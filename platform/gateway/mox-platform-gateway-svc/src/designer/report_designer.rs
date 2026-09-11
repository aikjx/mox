//! 报表设计器模型
//!
//! 支持低代码报表设计，包含数据源、图表、表格、仪表盘等

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 报表定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDefinition {
    /// 报表 ID
    pub report_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 报表名称
    pub name: String,
    /// 报表编码
    pub code: String,
    /// 报表分类：hr / finance / business / operation / custom
    pub category: String,
    /// 报表描述
    pub description: Option<String>,
    /// 报表类型：list / chart / dashboard / pivot
    pub report_type: String,
    /// 状态：draft / published / disabled
    pub status: String,
    /// 版本
    pub version: i32,
    /// 数据源配置
    pub data_sources: Vec<ReportDataSource>,
    /// 数据集配置
    pub datasets: Vec<ReportDataset>,
    /// 组件列表（图表/表格/指标卡等）
    pub components: Vec<ReportComponent>,
    /// 布局配置
    pub layout: ReportLayout,
    /// 过滤器配置
    pub filters: Vec<ReportFilter>,
    /// 权限配置
    pub permission_config: ReportPermissionConfig,
    /// 导出配置
    pub export_config: ReportExportConfig,
    /// 刷新策略
    pub refresh_config: ReportRefreshConfig,
    /// 创建人
    pub created_by: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 发布时间
    pub published_at: Option<String>,
}

/// 报表数据源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDataSource {
    /// 数据源 ID
    pub ds_id: String,
    /// 数据源名称
    pub name: String,
    /// 数据源类型：mysql / postgresql / oracle / sqlserver / api / csv / excel / internal
    pub ds_type: String,
    /// 连接配置
    pub connection: HashMap<String, String>,
    /// 状态
    pub status: String,
}

/// 报表数据集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDataset {
    /// 数据集 ID
    pub dataset_id: String,
    /// 数据集名称
    pub name: String,
    /// 数据源 ID
    pub ds_id: String,
    /// 查询类型：sql / table / api / custom
    pub query_type: String,
    /// SQL 查询语句
    pub sql: Option<String>,
    /// 表名
    pub table_name: Option<String>,
    /// API 路径
    pub api_path: Option<String>,
    /// 字段列表
    pub fields: Vec<DatasetField>,
    /// 参数列表
    pub params: Vec<DatasetParam>,
    /// 缓存配置
    pub cache_config: Option<CacheConfig>,
}

/// 数据集字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetField {
    /// 字段名
    pub field_name: String,
    /// 显示名
    pub display_name: String,
    /// 字段类型：string / number / date / datetime / boolean
    pub field_type: String,
    /// 是否维度
    pub is_dimension: bool,
    /// 是否度量
    pub is_measure: bool,
    /// 聚合方式：sum / avg / count / max / min / distinct_count
    pub aggregation: Option<String>,
    /// 格式化
    pub format: Option<String>,
}

/// 数据集参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetParam {
    pub param_name: String,
    pub param_type: String,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
}

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: i64,
}

/// 报表组件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportComponent {
    /// 组件 ID
    pub component_id: String,
    /// 组件类型
    pub component_type: ComponentType,
    /// 组件标题
    pub title: String,
    /// 组件描述
    pub description: Option<String>,
    /// 数据集 ID
    pub dataset_id: String,
    /// 组件配置（根据类型不同）
    pub config: ComponentConfig,
    /// 组件位置
    pub position: ComponentPosition,
    /// 组件大小
    pub size: ComponentSize,
    /// 是否显示标题
    pub show_title: bool,
    /// 样式配置
    pub style: ComponentStyle,
}

/// 组件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    /// 指标卡
    MetricCard,
    /// 表格
    Table,
    /// 透视表
    PivotTable,
    /// 折线图
    LineChart,
    /// 柱状图
    BarChart,
    /// 饼图
    PieChart,
    /// 环形图
    DonutChart,
    /// 面积图
    AreaChart,
    /// 散点图
    ScatterChart,
    /// 雷达图
    RadarChart,
    /// 漏斗图
    FunnelChart,
    /// 仪表盘
    GaugeChart,
    /// 热力图
    Heatmap,
    /// 地图
    Map,
    /// 进度条
    ProgressBar,
    /// 排名列表
    RankList,
    /// 文本
    Text,
    /// 图片
    Image,
    /// 分割线
    Divider,
    /// 标签页
    Tabs,
}

impl ComponentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MetricCard => "metric_card",
            Self::Table => "table",
            Self::PivotTable => "pivot_table",
            Self::LineChart => "line_chart",
            Self::BarChart => "bar_chart",
            Self::PieChart => "pie_chart",
            Self::DonutChart => "donut_chart",
            Self::AreaChart => "area_chart",
            Self::ScatterChart => "scatter_chart",
            Self::RadarChart => "radar_chart",
            Self::FunnelChart => "funnel_chart",
            Self::GaugeChart => "gauge_chart",
            Self::Heatmap => "heatmap",
            Self::Map => "map",
            Self::ProgressBar => "progress_bar",
            Self::RankList => "rank_list",
            Self::Text => "text",
            Self::Image => "image",
            Self::Divider => "divider",
            Self::Tabs => "tabs",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::MetricCard | Self::ProgressBar | Self::RankList => "指标",
            Self::Table | Self::PivotTable => "表格",
            Self::LineChart | Self::BarChart | Self::AreaChart | Self::ScatterChart => "趋势对比",
            Self::PieChart | Self::DonutChart | Self::FunnelChart => "占比分布",
            Self::RadarChart | Self::GaugeChart | Self::Heatmap => "多维分析",
            Self::Map => "地理",
            Self::Text | Self::Image | Self::Divider | Self::Tabs => "辅助",
        }
    }
}

/// 组件配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentConfig {
    /// X 轴字段
    pub x_field: Option<String>,
    /// Y 轴字段
    pub y_field: Option<String>,
    /// 系列字段
    pub series_field: Option<String>,
    /// 数值字段
    pub value_field: Option<String>,
    /// 分类字段
    pub category_field: Option<String>,
    /// 聚合方式
    pub aggregation: Option<String>,
    /// 排序
    pub sort: Option<String>,
    /// 限制条数
    pub limit: Option<i32>,
    /// 图表颜色
    pub colors: Option<Vec<String>>,
    /// 是否显示图例
    pub show_legend: Option<bool>,
    /// 是否显示标签
    pub show_label: Option<bool>,
    /// 是否显示网格
    pub show_grid: Option<bool>,
    /// 表格列配置
    pub columns: Option<Vec<TableColumn>>,
    /// 指标卡配置
    pub metric_config: Option<MetricCardConfig>,
}

/// 表格列配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub field: String,
    pub title: String,
    pub width: Option<i32>,
    pub align: Option<String>,
    pub format: Option<String>,
    pub sortable: Option<bool>,
    pub filterable: Option<bool>,
}

/// 指标卡配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricCardConfig {
    pub metric_field: String,
    pub metric_name: String,
    pub unit: Option<String>,
    pub precision: Option<i32>,
    pub trend_field: Option<String>,
    pub target_value: Option<f64>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

/// 组件位置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPosition {
    pub x: i32,
    pub y: i32,
}

/// 组件大小
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSize {
    pub width: i32,
    pub height: i32,
}

/// 组件样式
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentStyle {
    pub background_color: Option<String>,
    pub border_radius: Option<i32>,
    pub padding: Option<i32>,
    pub title_color: Option<String>,
    pub title_font_size: Option<i32>,
}

/// 报表布局
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportLayout {
    /// 布局类型：grid / canvas / responsive
    pub layout_type: String,
    /// 列数
    pub columns: i32,
    /// 行高
    pub row_height: i32,
    /// 间距
    pub gutter: i32,
    /// 背景色
    pub background_color: Option<String>,
}

impl Default for ReportLayout {
    fn default() -> Self {
        Self {
            layout_type: "grid".to_string(),
            columns: 24,
            row_height: 40,
            gutter: 16,
            background_color: None,
        }
    }
}

/// 报表过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub filter_id: String,
    pub field: String,
    pub label: String,
    pub filter_type: String,
    pub default_value: Option<serde_json::Value>,
    pub options: Option<Vec<serde_json::Value>>,
    pub required: bool,
    pub affected_components: Vec<String>,
}

/// 报表权限配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportPermissionConfig {
    /// 可见角色 ID 列表
    pub visible_roles: Vec<String>,
    /// 可导出角色 ID 列表
    pub export_roles: Vec<String>,
    /// 数据权限：all / dept / dept_and_children / self / custom
    pub data_scope: String,
    /// 行级权限表达式
    pub row_level_expr: Option<String>,
    /// 列级权限（隐藏字段）
    pub hidden_fields: Vec<String>,
}

/// 报表导出配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportExportConfig {
    pub allow_excel: bool,
    pub allow_pdf: bool,
    pub allow_csv: bool,
    pub allow_image: bool,
    pub watermark: bool,
    pub watermark_text: Option<String>,
}

/// 报表刷新策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReportRefreshConfig {
    /// 刷新方式：manual / auto / scheduled
    pub refresh_type: String,
    /// 自动刷新间隔（秒）
    pub auto_interval: Option<i64>,
    /// 定时刷新 Cron 表达式
    pub cron_expression: Option<String>,
}

/// 创建报表定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReportDefinitionRequest {
    pub name: String,
    pub code: String,
    pub category: String,
    pub report_type: String,
    pub description: Option<String>,
    pub data_sources: Vec<ReportDataSource>,
    pub datasets: Vec<ReportDataset>,
    pub components: Vec<ReportComponent>,
    pub layout: Option<ReportLayout>,
    pub filters: Option<Vec<ReportFilter>>,
    pub permission_config: Option<ReportPermissionConfig>,
    pub export_config: Option<ReportExportConfig>,
    pub refresh_config: Option<ReportRefreshConfig>,
}

/// 更新报表定义请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReportDefinitionRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub data_sources: Option<Vec<ReportDataSource>>,
    pub datasets: Option<Vec<ReportDataset>>,
    pub components: Option<Vec<ReportComponent>>,
    pub layout: Option<ReportLayout>,
    pub filters: Option<Vec<ReportFilter>>,
    pub permission_config: Option<ReportPermissionConfig>,
    pub export_config: Option<ReportExportConfig>,
    pub refresh_config: Option<ReportRefreshConfig>,
}

/// 支持的组件类型列表
pub fn supported_component_types() -> Vec<ComponentType> {
    vec![
        ComponentType::MetricCard,
        ComponentType::Table,
        ComponentType::PivotTable,
        ComponentType::LineChart,
        ComponentType::BarChart,
        ComponentType::PieChart,
        ComponentType::DonutChart,
        ComponentType::AreaChart,
        ComponentType::ScatterChart,
        ComponentType::RadarChart,
        ComponentType::FunnelChart,
        ComponentType::GaugeChart,
        ComponentType::Heatmap,
        ComponentType::Map,
        ComponentType::ProgressBar,
        ComponentType::RankList,
        ComponentType::Text,
        ComponentType::Image,
        ComponentType::Divider,
        ComponentType::Tabs,
    ]
}
