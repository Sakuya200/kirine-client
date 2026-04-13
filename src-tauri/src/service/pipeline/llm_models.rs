#[derive(Debug, Clone)]
pub(crate) struct LlmModelDefinition {
    pub display_name: &'static str,
    pub artifacts_dir: &'static str,
    pub python_script_dir: &'static str,
    pub required_model_name_list: Vec<&'static str>,
    pub required_model_repo_id_list: Vec<&'static str>,
}
