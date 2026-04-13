import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import shutil
import sys
from types import SimpleNamespace

target_speaker_embedding = None


@dataclass
class TrainingRuntimeOptions:
    is_cpu: bool
    accelerator_kwargs: dict[str, object]
    model_load_kwargs: dict[str, object]


@dataclass
class TrainingPipelineContext:
    runtime: TrainingRuntimeOptions
    accelerator: object
    qwen3tts: object
    model: object
    optimizer: object
    train_dataloader: object
    train_data: list[dict[str, object]]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--init_model_path",
        "--init-model-path",
        dest="init_model_path",
        type=str,
        default="Qwen/Qwen3-TTS-12Hz-1.7B-Base",
    )
    parser.add_argument(
        "--output_model_path",
        "--output-model-path",
        dest="output_model_path",
        type=str,
        default="output",
    )
    parser.add_argument("--train_jsonl", "--train-jsonl", dest="train_jsonl", type=str, required=True)
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--batch_size", "--batch-size", dest="batch_size", type=int, default=2)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--num_epochs", "--num-epochs", dest="num_epochs", type=int, default=3)
    parser.add_argument("--speaker_name", "--speaker-name", dest="speaker_name", type=str, default="speaker_test")
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="flash_attention_2",
    )
    return parser.parse_args(argv)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def normalize_speaker_name(speaker_name: str) -> str:
    normalized_name = speaker_name.strip().lower()
    if not normalized_name:
        raise ValueError("speaker_name must not be empty")
    return normalized_name


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


def load_training_dependencies() -> SimpleNamespace:
    ensure_src_root_on_path()

    import torch
    from accelerate import Accelerator
    from qwen3_tts.dataset import TTSDataset
    from qwen_tts.inference.qwen3_tts_model import Qwen3TTSModel
    from safetensors.torch import save_file
    from torch.optim import AdamW
    from torch.utils.data import DataLoader
    from transformers import AutoConfig

    return SimpleNamespace(
        torch=torch,
        Accelerator=Accelerator,
        TTSDataset=TTSDataset,
        Qwen3TTSModel=Qwen3TTSModel,
        save_file=save_file,
        AdamW=AdamW,
        DataLoader=DataLoader,
        AutoConfig=AutoConfig,
    )


def build_runtime_options(args: argparse.Namespace, torch_module) -> TrainingRuntimeOptions:
    accelerator_kwargs: dict[str, object] = {
        "gradient_accumulation_steps": 4,
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
    )


def load_training_rows(train_jsonl: str) -> list[dict[str, object]]:
    with open(train_jsonl, "r", encoding="utf-8") as file:
        train_data = file.readlines()
    return [json.loads(line) for line in train_data if line.strip()]


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
    config = deps.AutoConfig.from_pretrained(args.init_model_path)
    train_data = load_training_rows(args.train_jsonl)

    dataset = deps.TTSDataset(train_data, qwen3tts.processor, config)
    train_dataloader = deps.DataLoader(
        dataset,
        batch_size=args.batch_size,
        shuffle=True,
        collate_fn=dataset.collate_fn,
    )

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
        train_dataloader=train_dataloader,
        train_data=train_data,
    )


def get_model_device(model) -> object:
    if hasattr(model, "device"):
        return model.device
    first_param = next(model.parameters())
    return first_param.device


def get_model_dtype(model, default_dtype) -> object:
    if hasattr(model, "dtype"):
        return model.dtype
    first_param = next(model.parameters())
    return getattr(first_param, "dtype", default_dtype)


def run_training(args: argparse.Namespace, dependencies: SimpleNamespace | None = None) -> None:
    global target_speaker_embedding
    target_speaker_embedding = None

    deps = dependencies or load_training_dependencies()
    context = initialize_training_pipeline(args, deps)
    speaker_name = normalize_speaker_name(args.speaker_name)
    accelerator = context.accelerator
    model = context.model
    optimizer = context.optimizer
    train_dataloader = context.train_dataloader
    model_device = get_model_device(model)
    model_dtype = get_model_dtype(model, context.runtime.model_load_kwargs["torch_dtype"])

    num_epochs = args.num_epochs
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

                speaker_embedding = model.speaker_encoder(ref_mels.to(model_device).to(model_dtype)).detach()
                if target_speaker_embedding is None:
                    target_speaker_embedding = speaker_embedding

                input_text_ids = input_ids[:, :, 0]
                input_codec_ids = input_ids[:, :, 1]

                input_text_embedding = model.talker.model.text_embedding(input_text_ids) * text_embedding_mask
                input_codec_embedding = model.talker.model.codec_embedding(input_codec_ids) * codec_embedding_mask
                input_codec_embedding[:, 6, :] = speaker_embedding

                input_embeddings = input_text_embedding + input_codec_embedding

                for i in range(1, 16):
                    codec_i_embedding = model.talker.code_predictor.get_input_embeddings()[i - 1](codec_ids[:, :, i])
                    codec_i_embedding = codec_i_embedding * codec_mask.unsqueeze(-1)
                    input_embeddings = input_embeddings + codec_i_embedding

                outputs = model.talker(
                    inputs_embeds=input_embeddings[:, :-1, :],
                    attention_mask=attention_mask[:, :-1],
                    labels=codec_0_labels[:, 1:],
                    output_hidden_states=True,
                )

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
                print(f"Epoch {epoch} | Step {step} | Loss: {loss.item():.4f}")

        if accelerator.is_main_process:
            output_dir = os.path.join(args.output_model_path, f"checkpoint-epoch-{epoch}")
            shutil.copytree(args.init_model_path, output_dir, dirs_exist_ok=True)

            input_config_file = os.path.join(args.init_model_path, "config.json")
            output_config_file = os.path.join(output_dir, "config.json")
            with open(input_config_file, "r", encoding="utf-8") as f:
                config_dict = json.load(f)
            config_dict["tts_model_type"] = "custom_voice"
            talker_config = config_dict.get("talker_config", {})
            talker_config["spk_id"] = {
                speaker_name: 3000,
            }
            talker_config["spk_is_dialect"] = {
                speaker_name: False,
            }
            config_dict["talker_config"] = talker_config

            with open(output_config_file, "w", encoding="utf-8") as f:
                json.dump(config_dict, f, indent=2, ensure_ascii=False)

            unwrapped_model = accelerator.unwrap_model(model)
            state_dict = {k: v.detach().to("cpu") for k, v in unwrapped_model.state_dict().items()}

            drop_prefix = "speaker_encoder"
            keys_to_drop = [k for k in state_dict.keys() if k.startswith(drop_prefix)]
            for k in keys_to_drop:
                del state_dict[k]

            weight = state_dict["talker.model.codec_embedding.weight"]
            state_dict["talker.model.codec_embedding.weight"][3000] = target_speaker_embedding[0].detach().to(weight.device).to(weight.dtype)
            save_path = os.path.join(output_dir, "model.safetensors")
            deps.save_file(state_dict, save_path)


def train(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    run_training(args)


if __name__ == "__main__":
    train()