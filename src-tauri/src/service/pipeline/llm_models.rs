#[derive(Debug, Clone)]
pub(crate) struct LlmModelDefinition {
    pub display_name: &'static str,
    pub python_script_dir: &'static str,
}
