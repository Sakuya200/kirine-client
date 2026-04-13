use tauri::Wry;

mod settings;
mod speaker_info;
mod task_history;

pub use settings::EnvConfigState;

pub fn load_hooks(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        speaker_info::create_speaker_info,
        speaker_info::list_speaker_infos,
        speaker_info::update_speaker_info,
        speaker_info::delete_speaker_info,
        task_history::list_history_records,
        task_history::get_history_record,
        task_history::get_text_to_speech_audio,
        task_history::get_voice_clone_audio,
        task_history::save_text_to_speech_audio_as,
        task_history::save_voice_clone_audio_as,
        task_history::save_model_training_template_as,
        task_history::delete_history_record,
        task_history::create_text_to_speech_task,
        task_history::create_model_training_task,
        task_history::create_voice_clone_task,
        settings::get_settings_config,
        settings::save_settings_config
    ])
}
