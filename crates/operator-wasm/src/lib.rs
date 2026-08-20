//! # WASM插件系统
//!
//! 实现WASM格式算子插件的加载和执行
//! 支持热加载、类型检查、资源隔离

use operator_core::operator::Operator;
use operator_core::resource::ResourceCost;
use operator_core::state::StateVector;
use operator_core::types::{builtin, TypeCheck, TypeIdentifier};
use operator_core::{
    ExecutionContext, OperatorError, OperatorMetadata, Result,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use wasmer::{imports, Instance, Module, Store, Value};

/// WASM算子插件
pub struct WasmOperator {
    metadata: OperatorMetadata,
    module_bytes: Vec<u8>,
}

impl WasmOperator {
    /// 从文件加载WASM算子
    pub fn from_file(path: impl AsRef<Path>, metadata: OperatorMetadata) -> Result<Self> {
        let module_bytes = std::fs::read(path)
            .map_err(|e| OperatorError::WasmError(format!("读取WASM文件失败: {}", e)))?;
        Ok(Self {
            metadata,
            module_bytes,
        })
    }

    /// 从字节加载WASM算子
    pub fn from_bytes(bytes: Vec<u8>, metadata: OperatorMetadata) -> Self {
        Self {
            metadata,
            module_bytes: bytes,
        }
    }

    /// 在WASM沙箱中执行算子
    fn execute_wasm(&self, input: &[f64]) -> Result<Vec<f64>> {
        // 创建WASM store和module
        let mut store = Store::default();
        let module = Module::new(&store, &self.module_bytes)
            .map_err(|e| OperatorError::WasmError(format!("编译WASM模块失败: {}", e)))?;

        // 导入函数
        let import_object = imports! {};
        let instance = Instance::new(&mut store, &module, &import_object)
            .map_err(|e| OperatorError::WasmError(format!("实例化WASM失败: {}", e)))?;

        // 获取内存
        let memory = instance
            .exports
            .get_memory("memory")
            .map_err(|e| OperatorError::WasmError(format!("获取内存失败: {}", e)))?;

        // 获取apply函数
        let apply_func = instance
            .exports
            .get_function("operator_apply")
            .map_err(|e| OperatorError::WasmError(format!("获取operator_apply函数失败: {}", e)))?;

        let n = input.len();
        let input_offset = 0;
        let output_offset = n * 8;

        // 写入输入数据到WASM内存
        let input_bytes: Vec<u8> = input
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect();
        memory
            .view(&store)
            .write(input_offset as u64, &input_bytes)
            .map_err(|e| OperatorError::WasmError(format!("写入输入失败: {}", e)))?;

        // 调用apply函数
        let results = apply_func
            .call(
                &mut store,
                &[
                    Value::I32(input_offset),
                    Value::I32(output_offset as i32),
                    Value::I32(n as i32),
                ],
            )
            .map_err(|e| OperatorError::WasmError(format!("执行WASM函数失败: {}", e)))?;

        // 检查返回值
        if let Some(Value::I32(ret)) = results.first() {
            if *ret != 0 {
                return Err(OperatorError::WasmError(format!(
                    "WASM算子执行错误，返回码: {}",
                    ret
                )));
            }
        }

        // 读取输出数据
        let mut output_bytes = vec![0u8; n * 8];
        memory
            .view(&store)
            .read(output_offset as u64, &mut output_bytes)
            .map_err(|e| OperatorError::WasmError(format!("读取输出失败: {}", e)))?;

        let output: Vec<f64> = output_bytes
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(output)
    }
}

impl Operator for WasmOperator {
    fn metadata(&self) -> OperatorMetadata {
        self.metadata.clone()
    }

    fn apply(&self, input: &StateVector, _ctx: &mut ExecutionContext) -> Result<StateVector> {
        let input_vec = input.to_vec();
        let output_vec = self.execute_wasm(&input_vec)?;
        Ok(StateVector::from_vec(output_vec))
    }
}

impl TypeCheck for WasmOperator {
    fn input_type(&self) -> TypeIdentifier {
        self.metadata.input_type.clone()
    }

    fn output_type(&self) -> TypeIdentifier {
        self.metadata.output_type.clone()
    }
}

/// WASM插件管理器
pub struct WasmPluginManager {
    plugins: HashMap<String, Arc<WasmOperator>>,
    plugin_dir: std::path::PathBuf,
}

impl WasmPluginManager {
    pub fn new(plugin_dir: impl AsRef<Path>) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }

    /// 加载目录下所有WASM插件
    pub fn load_all(&mut self) -> Result<()> {
        if !self.plugin_dir.exists() {
            std::fs::create_dir_all(&self.plugin_dir)
                .map_err(|e| OperatorError::WasmError(format!("创建插件目录失败: {}", e)))?;
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.plugin_dir)
            .map_err(|e| OperatorError::WasmError(format!("读取插件目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| OperatorError::WasmError(format!("读取目录项失败: {}", e)))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "wasm") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let metadata = OperatorMetadata {
                    id: format!("wasm-{}", name),
                    name: name.clone(),
                    version: "1.0.0".to_string(),
                    description: format!("WASM插件: {}", name),
                    input_type: builtin::state_vector_type(),
                    output_type: builtin::state_vector_type(),
                    resource_cost: ResourceCost::default(),
                    author: "WASM Plugin".to_string(),
                    tags: vec!["wasm".to_string(), "plugin".to_string()],
                };
                let plugin = WasmOperator::from_file(&path, metadata)?;
                self.plugins.insert(name, Arc::new(plugin));
                tracing::info!("加载WASM插件: {}", path.display());
            }
        }
        Ok(())
    }

    /// 获取插件
    pub fn get(&self, name: &str) -> Option<Arc<WasmOperator>> {
        self.plugins.get(name).cloned()
    }

    /// 列出所有插件
    pub fn list(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// 卸载插件
    pub fn unload(&mut self, name: &str) -> bool {
        self.plugins.remove(name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_manager_creation() {
        let manager = WasmPluginManager::new("/tmp/test-plugins");
        assert!(manager.list().is_empty());
    }
}
