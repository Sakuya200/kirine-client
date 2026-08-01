use tauri::Wry;

mod model_info;
mod settings;
mod speaker_info;
pub(crate) mod streaming;
mod task_history;

pub use settings::{EnvConfigState, UiConfigState};

pub fn load_hooks(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        speaker_info::create_speaker_info,
        speaker_info::import_model_as_speaker,
        speaker_info::list_speaker_infos,
        speaker_info::update_speaker_info,
        speaker_info::delete_speaker_info,
        model_info::list_model_infos,
        model_info::get_device_type,
        model_info::install_model,
        model_info::uninstall_model,
        model_info::set_model_current_device,
        task_history::list_history_records,
        task_history::get_history_record,
        task_history::get_generated_audio,
        task_history::save_generated_audio_as,
        task_history::save_model_training_template_as,
        task_history::delete_history_record,
        task_history::create_text_to_speech_task,
        task_history::create_model_training_task,
        task_history::cancel_history_task,
        task_history::create_voice_clone_task,
        task_history::create_voice_design_task,
        streaming::create_streaming_speech_task,
        streaming::send_streaming_message,
        streaming::cancel_streaming_task,
        streaming::get_streaming_replay_snapshot,
        settings::get_settings_config,
        settings::get_ui_config,
        settings::save_settings_config
    ])
}
