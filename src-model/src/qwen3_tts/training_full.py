import argparse
import json
import os
import shutil
from types import SimpleNamespace

from huggingface_hub import save_torch_state_dict

from qwen3_tts.training_common import (
    TrainingPipelineContext,
    TrainingRuntimeOptions,
    add_common_training_args,
    build_train_dataloader,
    enable_gradient_checkpointing,
    is_cpu_device,
    load_training_dependencies,
)


CHECKPOINT_MAX_SHARD_SIZE = "1GB"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    return add_common_training_args(argparse.ArgumentParser()).parse_args(argv)


def disable_use_cache_for_training(model) -> None:
    for candidate in (
        model,
        getattr(model, "talker", None),
        getattr(getattr(model, "talker", None), "model", None),
        getattr(model, "code_predictor", None),
    ):
        if candidate is None:
            continue
        config = getattr(candidate, "config", None)
        if config is not None and hasattr(config, "use_cache"):
            config.use_cache = False


def build_checkpoint_state_dict(unwrapped_model, target_speaker_embedding) -> dict[str, object]:
    state_dict = {}
    for key, value in unwrapped_model.state_dict().items():
        if key.startswith("speaker_encoder"):
            continue
        state_dict[key] = value.detach().to("cpu")

    codec_embedding_key = "talker.model.codec_embedding.weight"
    codec_embedding = state_dict[codec_embedding_key].clone()
    codec_embedding[3000] = target_speaker_embedding[0].detach().to("cpu").to(codec_embedding.dtype)
    state_dict[codec_embedding_key] = codec_embedding
    return state_dict


def save_training_checkpoint(args: argparse.Namespace, accelerator, model, target_speaker_embedding, epoch: int) -> None:
    output_dir = os.path.join(args.output_model_path, f"checkpoint-epoch-{epoch}")
    shutil.copytree(args.init_model_path, output_dir, dirs_exist_ok=True)

    input_config_file = os.path.join(args.init_model_path, "config.json")
    output_config_file = os.path.join(output_dir, "config.json")
    with open(input_config_file, "r", encoding="utf-8") as file:
        config_dict = json.load(file)
    config_dict["tts_model_type"] = "custom_voice"
    talker_config = config_dict.get("talker_config", {})
    talker_config["spk_id"] = {
        args.speaker_name: 3000,
    }
    talker_config["spk_is_dialect"] = {
        args.speaker_name: False,
    }
    config_dict["talker_config"] = talker_config

    with open(output_config_file, "w", encoding="utf-8") as file:
        json.dump(config_dict, file, indent=2, ensure_ascii=False)

    unwrapped_model = accelerator.unwrap_model(model)
    state_dict = build_checkpoint_state_dict(unwrapped_model, target_speaker_embedding)
    save_torch_state_dict(
        state_dict,
        output_dir,
        max_shard_size=CHECKPOINT_MAX_SHARD_SIZE,
        safe_serialization=True,
        is_main_process=accelerator.is_main_process,
        shared_tensors_to_discard=getattr(unwrapped_model, "_tied_weights_keys", None),
    )


def build_runtime_options(args: argparse.Namespace, torch_module) -> TrainingRuntimeOptions:
    accelerator_kwargs: dict[str, object] = {
        "gradient_accumulation_steps": args.gradient_accumulation_steps,
        "log_with": "tensorboard",
    }
    if args.logging_dir:
        accelerator_kwargs["project_dir"] = args.logging_dir

    if is_cpu_device(args.device):
        return TrainingRuntimeOptions(
            is_cpu=True,
            accelerator_kwargs={
                **accelerator_kwargs,
                "mixed_precision": "no",
                "cpu": True,
            },
            model_load_kwargs={
                "torch_dtype": torch_module.float32,
            },
            mode_label="full fine-tune",
        )

    return TrainingRuntimeOptions(
        is_cpu=False,
        accelerator_kwargs={
            **accelerator_kwargs,
            "mixed_precision": "bf16",
        },
        model_load_kwargs={
            "torch_dtype": torch_module.bfloat16,
            "attn_implementation": args.attn_implementation,
        },
        mode_label="full fine-tune",
    )


def initialize_training_pipeline(
    args: argparse.Namespace,
    dependencies: SimpleNamespace | None = None,
) -> TrainingPipelineContext:
    deps = dependencies or load_training_dependencies()
    runtime = build_runtime_options(args, deps.torch)
    accelerator = deps.Accelerator(**runtime.accelerator_kwargs)

    qwen3tts = deps.Qwen3TTSModel.from_pretrained(
        args.init_model_path,
        **runtime.model_load_kwargs,
    )

    if args.enable_gradient_checkpointing:
        disable_use_cache_for_training(qwen3tts.model)
        enable_gradient_checkpointing(qwen3tts.model, require_input_grads=False)

    train_data, train_dataloader = build_train_dataloader(args, deps, qwen3tts)
    optimizer = deps.AdamW(qwen3tts.model.parameters(), lr=args.lr, weight_decay=0.01)
    model, optimizer, train_dataloader = accelerator.prepare(
        qwen3tts.model,
        optimizer,
        train_dataloader,
    )
    return TrainingPipelineContext(
        runtime=runtime,
        accelerator=accelerator,
        qwen3tts=qwen3tts,
        model=model,
        optimizer=optimizer,
        scheduler=None,
        train_dataloader=train_dataloader,
        train_data=train_data,
    )


def run_training(args: argparse.Namespace, dependencies: SimpleNamespace | None = None) -> None:
    deps = dependencies or load_training_dependencies()
    context = initialize_training_pipeline(args, deps)
    accelerator = context.accelerator
    model = context.model
    optimizer = context.optimizer
    train_dataloader = context.train_dataloader

    num_epochs = args.num_epochs
    target_speaker_embedding = None
    model.train()

    for epoch in range(num_epochs):
        for step, batch in enumerate(train_dataloader):
            with accelerator.accumulate(model):
                input_ids = batch["input_ids"]
                codec_ids = batch["codec_ids"]
                ref_mels = batch["ref_mels"]
                text_embedding_mask = batch["text_embedding_mask"]
                codec_embedding_mask = batch["codec_embedding_mask"]
                attention_mask = batch["attention_mask"]
                codec_0_labels = batch["codec_0_labels"]
                codec_mask = batch["codec_mask"]

                speaker_embedding = model.speaker_encoder(ref_mels.to(model.device).to(model.dtype)).detach()
                if target_speaker_embedding is None:
                    target_speaker_embedding = speaker_embedding

                input_text_ids = input_ids[:, :, 0]
                input_codec_ids = input_ids[:, :, 1]

                input_text_embedding = model.talker.model.text_embedding(input_text_ids) * text_embedding_mask
                input_codec_embedding = model.talker.model.codec_embedding(input_codec_ids) * codec_embedding_mask
                input_codec_embedding[:, 6, :] = speaker_embedding

                input_embeddings = input_text_embedding + input_codec_embedding

                for index in range(1, 16):
                    codec_i_embedding = model.talker.code_predictor.get_input_embeddings()[index - 1](codec_ids[:, :, index])
                    codec_i_embedding = codec_i_embedding * codec_mask.unsqueeze(-1)
                    input_embeddings = input_embeddings + codec_i_embedding

                talker_forward_kwargs = {
                    "inputs_embeds": input_embeddings[:, :-1, :],
                    "attention_mask": attention_mask[:, :-1],
                    "labels": codec_0_labels[:, 1:],
                    "output_hidden_states": True,
                }
                if args.enable_gradient_checkpointing:
                    talker_forward_kwargs["use_cache"] = False

                outputs = model.talker(**talker_forward_kwargs)

                hidden_states = outputs.hidden_states[0][-1]
                talker_hidden_states = hidden_states[codec_mask[:, :-1]]
                talker_codec_ids = codec_ids[codec_mask]

                _, sub_talker_loss = model.talker.forward_sub_talker_finetune(
                    talker_codec_ids,
                    talker_hidden_states,
                )

                loss = outputs.loss + 0.3 * sub_talker_loss

                accelerator.backward(loss)

                if accelerator.sync_gradients:
                    accelerator.clip_grad_norm_(model.parameters(), 1.0)

                optimizer.step()
                optimizer.zero_grad()

            if step % 10 == 0:
                accelerator.print(f"Epoch {epoch} | Step {step} | Loss: {loss.item():.4f}")

        if accelerator.is_main_process and epoch == num_epochs - 1:
            save_training_checkpoint(args, accelerator, model, target_speaker_embedding, epoch)


def train(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    run_training(args)


if __name__ == "__main__":
    train()