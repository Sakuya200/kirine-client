use crate::service::models::AppLanguage;

const QWEN3_TTS_APP_SUPPORTED_LANGUAGES: &[AppLanguage] = &[
    AppLanguage::Chinese,
    AppLanguage::English,
    AppLanguage::Japanese,
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct Qwen3TtsPresetSpeaker {
    pub name: &'static str,
    pub description: &'static str,
    pub languages: &'static [AppLanguage],
}

const QWEN3_TTS_PRESET_SPEAKERS: &[Qwen3TtsPresetSpeaker] = &[
    Qwen3TtsPresetSpeaker {
        name: "Vivian",
        description: "Bright, slightly edgy young female voice. Native language: Chinese.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Serena",
        description: "Warm, gentle young female voice. Native language: Chinese.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Uncle_Fu",
        description: "Seasoned male voice with a low, mellow timbre. Native language: Chinese.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Dylan",
        description: "Youthful Beijing male voice with a clear, natural timbre. Native language: Chinese (Beijing Dialect).",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Eric",
        description: "Lively Chengdu male voice with a slightly husky brightness. Native language: Chinese (Sichuan Dialect).",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Ryan",
        description: "Dynamic male voice with strong rhythmic drive. Native language: English.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Aiden",
        description: "Sunny American male voice with a clear midrange. Native language: English.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Ono_Anna",
        description: "Playful Japanese female voice with a light, nimble timbre. Native language: Japanese.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
    Qwen3TtsPresetSpeaker {
        name: "Sohee",
        description: "Warm Korean female voice with rich emotion. Native language: Korean.",
        languages: QWEN3_TTS_APP_SUPPORTED_LANGUAGES,
    },
];

pub(crate) fn qwen3_tts_preset_speakers() -> &'static [Qwen3TtsPresetSpeaker] {
    QWEN3_TTS_PRESET_SPEAKERS
}