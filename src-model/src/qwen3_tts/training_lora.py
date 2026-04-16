"""LoRA training placeholder module.

LoRA fine-tuning for Qwen3-TTS is temporarily disabled in this project.
This file is intentionally kept as a placeholder so the disabled status is explicit.
"""


def lora_unavailable_message() -> str:
    return "LoRA fine-tuning is temporarily disabled for Qwen3-TTS in this project."


def train(argv: list[str] | None = None) -> None:
    del argv
    raise RuntimeError(lora_unavailable_message())