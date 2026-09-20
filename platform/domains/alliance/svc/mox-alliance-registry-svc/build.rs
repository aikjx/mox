fn main() -> Result<(), Box<dyn std::error::Error>> {
    // gRPC 构建已启用，但代码有类型不匹配问题，暂时禁用
    // 修复后取消注释以下代码：
    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(false)
    //     .compile(&["proto/registry.proto"], &["proto"])?;
    Ok(())
}
