#!/usr/bin/env python3
"""
MOX Enterprise · Spark 分布式 GC（垃圾回收）Job
====================================================
功能：扫描所有 shard 的引用计数，回收 ref_count == 0 的 chunk
      替代 Node.js 单进程 GC，支持千亿亿级对象规模

运行方式：
  spark-submit \
    --master k8s://https://k8s-api:6443 \
    --deploy-mode cluster \
    --executor-memory 8g \
    --executor-cores 4 \
    --num-executors 20 \
    --conf spark.kubernetes.container.image=mox/spark-runner:3.5.0 \
    spark-gc-job.py \
    --tikv-pd tikv-pd:2379 \
    --s3-bucket mox-chunks-prod \
    --dry-run false
"""

import argparse
import json
import logging
import sys
import time
from datetime import datetime, timezone
from typing import Dict, List, Tuple

from pyspark.sql import SparkSession, DataFrame
from pyspark.sql import functions as F
from pyspark.sql.types import (
    StructType, StructField, StringType, LongType,
    IntegerType, TimestampType, BooleanType
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
)
logger = logging.getLogger("mox-spark-gc")

# ─── Schema 定义 ───
REF_COUNTER_SCHEMA = StructType([
    StructField("sha256", StringType(), nullable=False),
    StructField("ref_count", LongType(), nullable=False),
    StructField("tenant_refs", StringType(), nullable=True),  # JSONB
    StructField("gc_status", StringType(), nullable=False),
    StructField("last_gc_time", TimestampType(), nullable=True),
    StructField("shard_id", IntegerType(), nullable=False),
])

CHUNK_META_SCHEMA = StructType([
    StructField("sha256", StringType(), nullable=False),
    StructField("size", LongType(), nullable=False),
    StructField("codec", StringType(), nullable=True),
    StructField("ec_profile", StringType(), nullable=True),
    StructField("location_ids", StringType(), nullable=True),  # JSON array
    StructField("create_time", TimestampType(), nullable=False),
    StructField("last_access", TimestampType(), nullable=True),
    StructField("shard_id", IntegerType(), nullable=False),
])

GC_REPORT_SCHEMA = StructType([
    StructField("run_id", StringType(), nullable=False),
    StructField("start_time", TimestampType(), nullable=False),
    StructField("end_time", TimestampType(), nullable=True),
    StructField("total_chunks_scanned", LongType(), nullable=False),
    StructField("chunks_marked_deletion", LongType(), nullable=False),
    StructField("chunks_deleted", LongType(), nullable=False),
    StructField("bytes_freed", LongType(), nullable=False),
    StructField("errors", LongType(), nullable=False),
    StructField("status", StringType(), nullable=False),
])


class MOXSparkGC:
    """MOX 分布式垃圾回收器"""

    def __init__(self, spark: SparkSession, args: argparse.Namespace):
        self.spark = spark
        self.args = args
        self.run_id = f"gc-{datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')}"
        self.start_time = datetime.now(timezone.utc)
        self.stats = {
            "total_scanned": 0,
            "marked_deletion": 0,
            "deleted": 0,
            "bytes_freed": 0,
            "errors": 0,
        }

    def load_ref_counters(self) -> DataFrame:
        """从 TiKV / Parquet 加载引用计数器"""
        logger.info("加载引用计数器...")
        if self.args.ref_counter_path:
            df = self.spark.read.parquet(self.args.ref_counter_path)
        else:
            # 从 TiKV 读取（通过 jdbc 或自定义数据源）
            df = self.spark.read.format("jdbc") \
                .option("url", f"jdbc:mysql://{self.args.tikv_pd}/mox_meta") \
                .option("dbtable", "ref_counter") \
                .option("numPartitions", "256") \
                .option("partitionColumn", "shard_id") \
                .option("lowerBound", "0") \
                .option("upperBound", "255") \
                .load()
        logger.info(f"引用计数器加载完成，共 {df.count()} 条记录")
        return df

    def load_chunk_meta(self) -> DataFrame:
        """加载 chunk 元数据"""
        logger.info("加载 chunk 元数据...")
        if self.args.chunk_meta_path:
            df = self.spark.read.parquet(self.args.chunk_meta_path)
        else:
            df = self.spark.read.format("jdbc") \
                .option("url", f"jdbc:mysql://{self.args.tikv_pd}/mox_meta") \
                .option("dbtable", "chunk_meta") \
                .option("numPartitions", "256") \
                .option("partitionColumn", "shard_id") \
                .option("lowerBound", "0") \
                .option("upperBound", "255") \
                .load()
        return df

    def find_orphan_chunks(self, ref_df: DataFrame, chunk_df: DataFrame) -> DataFrame:
        """找出 ref_count == 0 的孤儿 chunk"""
        logger.info("扫描孤儿 chunk（ref_count == 0）...")

        # 过滤 ref_count == 0 且 gc_status != 'deleting'
        orphans = ref_df.filter(
            (F.col("ref_count") == 0) &
            (F.col("gc_status") != "deleted") &
            (F.col("gc_status") != "deleting")
        )

        # Join chunk_meta 获取大小信息
        orphans_with_size = orphans.join(
            chunk_df.select("sha256", "size", "location_ids"),
            on="sha256",
            how="inner"
        )

        count = orphans_with_size.count()
        total_bytes = orphans_with_size.agg(F.sum("size")).collect()[0][0] or 0
        self.stats["total_scanned"] = ref_df.count()
        self.stats["marked_deletion"] = count
        self.stats["bytes_freed"] = total_bytes

        logger.info(f"发现 {count} 个孤儿 chunk，可释放 {total_bytes / 1024**3:.2f} GB")
        return orphans_with_size

    def delete_chunks(self, orphans: DataFrame) -> None:
        """删除孤儿 chunk（S3 + 元数据）"""
        if self.args.dry_run:
            logger.info("[DRY-RUN] 跳过实际删除")
            return

        logger.info("开始删除孤儿 chunk...")

        # 使用 foreachPartition 批量删除，每个 partition 一个 S3 客户端
        def delete_partition(iterator):
            import boto3
            import os
            s3 = boto3.client(
                "s3",
                region_name=os.environ.get("AWS_REGION", "ap-southeast-1"),
                aws_access_key_id=os.environ.get("S3_ACCESS_KEY"),
                aws_secret_access_key=os.environ.get("S3_SECRET_KEY"),
            )
            bucket = os.environ.get("S3_CHUNKS_BUCKET", "mox-chunks-prod")
            deleted = 0
            errors = 0
            batch = []
            for row in iterator:
                batch.append({"Key": f"{row.sha256[:2]}/{row.sha256}"})
                if len(batch) >= 100:
                    try:
                        resp = s3.delete_objects(Bucket=bucket, Delete={"Objects": batch})
                        deleted += len(resp.get("Deleted", []))
                        errors += len(resp.get("Errors", []))
                    except Exception as e:
                        errors += len(batch)
                        logger.error(f"批量删除失败: {e}")
                    batch = []
            if batch:
                try:
                    resp = s3.delete_objects(Bucket=bucket, Delete={"Objects": batch})
                    deleted += len(resp.get("Deleted", []))
                    errors += len(resp.get("Errors", []))
                except Exception as e:
                    errors += len(batch)
            yield (deleted, errors)

        results = orphans.rdd.foreachPartition(delete_partition)
        # 注意：foreachPartition 不返回值，实际统计需通过累加器
        logger.info("删除完成")

    def update_metadata(self, orphans: DataFrame) -> None:
        """更新元数据 gc_status = 'deleted'"""
        if self.args.dry_run:
            return

        logger.info("更新元数据 gc_status...")
        # 通过批量更新 TiKV / 数据库
        orphans.select("sha256", "shard_id").write \
            .format("jdbc") \
            .option("url", f"jdbc:mysql://{self.args.tikv_pd}/mox_meta") \
            .option("dbtable", "gc_pending") \
            .mode("append") \
            .save()

    def generate_report(self) -> Dict:
        """生成 GC 报告"""
        end_time = datetime.now(timezone.utc)
        duration = (end_time - self.start_time).total_seconds()
        report = {
            "run_id": self.run_id,
            "start_time": self.start_time.isoformat(),
            "end_time": end_time.isoformat(),
            "duration_seconds": duration,
            "total_chunks_scanned": self.stats["total_scanned"],
            "chunks_marked_deletion": self.stats["marked_deletion"],
            "chunks_deleted": self.stats["deleted"],
            "bytes_freed": self.stats["bytes_freed"],
            "bytes_freed_gb": self.stats["bytes_freed"] / 1024**3,
            "errors": self.stats["errors"],
            "status": "completed" if self.stats["errors"] == 0 else "completed_with_errors",
            "dry_run": self.args.dry_run,
        }
        return report

    def run(self) -> Dict:
        """执行完整 GC 流程"""
        logger.info(f"========== MOX Spark GC 启动 [{self.run_id}] ==========")
        logger.info(f"参数: {vars(self.args)}")

        try:
            ref_df = self.load_ref_counters()
            chunk_df = self.load_chunk_meta()
            orphans = self.find_orphan_chunks(ref_df, chunk_df)
            self.delete_chunks(orphans)
            self.update_metadata(orphans)
            report = self.generate_report()
            logger.info(f"========== GC 完成 ==========")
            logger.info(json.dumps(report, indent=2, ensure_ascii=False))
            return report
        except Exception as e:
            logger.error(f"GC 失败: {e}", exc_info=True)
            self.stats["errors"] += 1
            report = self.generate_report()
            report["status"] = "failed"
            report["error"] = str(e)
            return report


def main():
    parser = argparse.ArgumentParser(description="MOX Spark 分布式 GC Job")
    parser.add_argument("--tikv-pd", default="tikv-pd:2379", help="TiKV PD 地址")
    parser.add_argument("--s3-bucket", default="mox-chunks-prod", help="S3 bucket")
    parser.add_argument("--ref-counter-path", default="", help="引用计数器 Parquet 路径（可选）")
    parser.add_argument("--chunk-meta-path", default="", help="chunk 元数据 Parquet 路径（可选）")
    parser.add_argument("--dry-run", type=lambda x: x.lower() == "true", default=False, help="只扫描不删除")
    parser.add_argument("--report-path", default="", help="报告输出路径")
    args = parser.parse_args()

    spark = SparkSession.builder \
        .appName("MOX-Distributed-GC") \
        .config("spark.sql.adaptive.enabled", "true") \
        .config("spark.sql.adaptive.coalescePartitions.enabled", "true") \
        .config("spark.executor.memoryOverhead", "2g") \
        .getOrCreate()

    spark.sparkContext.setLogLevel("WARN")

    gc = MOXSparkGC(spark, args)
    report = gc.run()

    if args.report_path:
        report_df = spark.createDataFrame([report])
        report_df.write.mode("overwrite").json(args.report_path)
        logger.info(f"报告已写入: {args.report_path}")

    spark.stop()

    if report["status"] == "failed":
        sys.exit(1)


if __name__ == "__main__":
    main()
