use super::ScriptType;
use crate::script::exec::executor::ExecutorInstance;
use crate::script::exec::strategy::node::NodeStrategy;
use crate::script::exec::strategy::python::PythonStrategy;
use crate::script::exec::strategy::shell::ShellStrategy;
use niupanel_common::error::Result;
use std::collections::HashMap;

pub struct ExecutorFactory;

impl ExecutorFactory {
    pub fn create(
        env_type: &str,
        version: Option<&str>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<ExecutorInstance> {
        let script_type = ScriptType::try_from_str(env_type)?;

        match script_type {
            ScriptType::Python => Ok(ExecutorInstance::Python(PythonStrategy::new(
                version, mirrors,
            ))),
            ScriptType::NodeJs => Ok(ExecutorInstance::Node(NodeStrategy::new(version, mirrors))),
            ScriptType::Shell => Ok(ExecutorInstance::Shell(ShellStrategy::new())),
        }
    }
}
