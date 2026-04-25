import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]

if str(SRC_MODEL_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_MODEL_ROOT))


def load_module():
    module_name = "vox_cpm2.training"
    sys.modules.pop(module_name, None)
    sys.modules.setdefault("yaml", SimpleNamespace(safe_dump=lambda *args, **kwargs: None))
    return importlib.import_module(module_name)


def load_runtime_module():
    module_name = "vox_cpm2"
    sys.modules.pop(module_name, None)
    return importlib.import_module(module_name)


class VoxCpm2TrainingScriptResolutionTests(unittest.TestCase):
    def test_resolve_train_script_path_uses_localized_script_when_available(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_training_path = temp_root / "src-model" / "vox_cpm2" / "training.py"
            fake_training_path.parent.mkdir(parents=True, exist_ok=True)
            fake_training_path.write_text("# stub", encoding="utf-8")

            local_script = temp_root / "src-model" / "vox_cpm2" / "train_voxcpm_finetune.py"
            local_script.write_text("# train", encoding="utf-8")

            with patch.object(module, "__file__", str(fake_training_path)), patch.object(
                module.importlib.util, "find_spec", return_value=None
            ):
                resolved = module.resolve_train_script_path("")

            self.assertEqual(resolved, local_script.resolve())

    def test_resolve_train_script_path_raises_when_no_local_or_installed_script_exists(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_training_path = temp_root / "src-model" / "vox_cpm2" / "training.py"
            fake_training_path.parent.mkdir(parents=True, exist_ok=True)
            fake_training_path.write_text("# stub", encoding="utf-8")

            with patch.object(module, "__file__", str(fake_training_path)), patch.object(
                module.importlib.util, "find_spec", return_value=None
            ):
                with self.assertRaises(FileNotFoundError) as context:
                    module.resolve_train_script_path("")

            self.assertIn("Expected bundled script", str(context.exception))

    def test_resolve_train_script_path_uses_installed_package_script_as_fallback(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_training_path = temp_root / "src-model" / "vox_cpm2" / "training.py"
            fake_training_path.parent.mkdir(parents=True, exist_ok=True)
            fake_training_path.write_text("# stub", encoding="utf-8")

            site_packages_root = temp_root / "venv" / "Lib" / "site-packages"
            package_root = site_packages_root / "voxcpm"
            package_root.mkdir(parents=True, exist_ok=True)
            package_init = package_root / "__init__.py"
            package_init.write_text("# init", encoding="utf-8")
            installed_script = site_packages_root / "scripts" / "train_voxcpm_finetune.py"
            installed_script.parent.mkdir(parents=True, exist_ok=True)
            installed_script.write_text("# installed train", encoding="utf-8")
            fake_spec = SimpleNamespace(origin=str(package_init))

            with patch.object(module, "__file__", str(fake_training_path)), patch.object(
                module.importlib.util, "find_spec", return_value=fake_spec
            ):
                resolved = module.resolve_train_script_path("")

            self.assertEqual(resolved, installed_script.resolve())

    def test_estimate_training_schedule_accounts_for_gradient_accumulation(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            manifest_path = temp_root / "train.jsonl"
            rows = [
                {"audio": f"sample-{index}.wav", "ref_audio": "ref.wav", "text": f"line-{index}"}
                for index in range(166)
            ]
            manifest_path.write_text(
                "\n".join(json.dumps(row, ensure_ascii=False) for row in rows) + "\n",
                encoding="utf-8",
            )

            schedule = module.estimate_training_schedule(
                manifest_path,
                batch_size=2,
                gradient_accumulation_steps=4,
                num_epochs=4,
            )

        self.assertEqual(schedule["sample_count"], 166)
        self.assertEqual(schedule["effective_batch_size"], 8)
        self.assertEqual(schedule["steps_per_epoch"], 21)
        self.assertEqual(schedule["total_steps"], 84)

    def test_build_training_config_uses_schedule_derived_steps(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            manifest_path = temp_root / "train.jsonl"
            output_dir = temp_root / "output"
            rows = [
                {"audio": f"sample-{index}.wav", "ref_audio": "ref.wav", "text": f"line-{index}"}
                for index in range(9)
            ]
            manifest_path.write_text(
                "\n".join(json.dumps(row, ensure_ascii=False) for row in rows) + "\n",
                encoding="utf-8",
            )

            args = module.parse_args(
                [
                    "--train-jsonl",
                    str(manifest_path),
                    "--output-model-path",
                    str(output_dir),
                    "--init-model-path",
                    str(temp_root),
                    "--batch-size",
                    "2",
                    "--num-epochs",
                    "3",
                    "--gradient-accumulation-steps",
                    "2",
                ]
            )

            config = module.build_training_config(args, manifest_path, output_dir)

        self.assertEqual(config["num_iters"], 9)
        self.assertEqual(config["max_steps"], 9)
        self.assertEqual(config["warmup_steps"], 1)
        self.assertEqual(config["log_interval"], 3)
        self.assertGreaterEqual(config["num_workers"], 2)

    def test_resolve_warmup_steps_stays_below_max_steps_for_multi_step_training(self):
        module = load_module()

        self.assertEqual(module.resolve_warmup_steps(1), 1)
        self.assertEqual(module.resolve_warmup_steps(2), 1)
        self.assertEqual(module.resolve_warmup_steps(9), 1)
        self.assertEqual(module.resolve_warmup_steps(84), 9)
        self.assertEqual(module.resolve_warmup_steps(300), 30)


class VoxCpm2RuntimeMetadataResolutionTests(unittest.TestCase):
    def test_resolve_runtime_target_includes_checkpoint_lora_config(self):
        module = load_runtime_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_init_path = temp_root / "src-model" / "vox_cpm2" / "__init__.py"
            fake_init_path.parent.mkdir(parents=True, exist_ok=True)
            fake_init_path.write_text("# stub", encoding="utf-8")

            base_model_dir = temp_root / "src-model" / "base-models" / "VoxCPM2"
            base_model_dir.mkdir(parents=True, exist_ok=True)
            runtime_model_dir = temp_root / "models" / "13_hare"
            checkpoint_dir = runtime_model_dir / "checkpoints" / "lora" / "latest"
            checkpoint_dir.mkdir(parents=True, exist_ok=True)
            (checkpoint_dir / "lora_config.json").write_text(
                json.dumps(
                    {
                        "base_model": str(base_model_dir),
                        "lora_config": {
                            "enable_lm": True,
                            "enable_dit": True,
                            "enable_proj": False,
                            "r": 16,
                            "alpha": 32,
                            "dropout": 0.0,
                        },
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            metadata_path = runtime_model_dir / module.RUNTIME_METADATA_FILE_NAME
            metadata_path.write_text(
                json.dumps(
                    {
                        "trainingMode": "lora",
                        "baseModelPath": str(base_model_dir),
                        "latestCheckpointPath": str(checkpoint_dir),
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )

            with patch.object(module, "__file__", str(fake_init_path)), patch.object(
                module, "SRC_MODEL_ROOT", temp_root / "src-model"
            ):
                runtime_target = module.resolve_runtime_target(str(runtime_model_dir))

        self.assertEqual(runtime_target.model_path, str(base_model_dir.resolve()))
        self.assertEqual(
            runtime_target.load_kwargs["lora_weights_path"],
            str(checkpoint_dir.resolve()),
        )
        self.assertEqual(runtime_target.load_kwargs["lora_config_dict"]["r"], 16)
        self.assertEqual(runtime_target.load_kwargs["lora_config_dict"]["alpha"], 32)

    def test_resolve_runtime_target_uses_relative_fallbacks_when_absolute_paths_move(self):
        module = load_runtime_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_init_path = temp_root / "src-model" / "vox_cpm2" / "__init__.py"
            fake_init_path.parent.mkdir(parents=True, exist_ok=True)
            fake_init_path.write_text("# stub", encoding="utf-8")

            base_model_dir = temp_root / "src-model" / "base-models" / "VoxCPM2"
            base_model_dir.mkdir(parents=True, exist_ok=True)
            runtime_model_dir = temp_root / "models" / "13_hare"
            checkpoint_dir = runtime_model_dir / "checkpoints" / "lora" / "latest"
            checkpoint_dir.mkdir(parents=True, exist_ok=True)
            metadata_path = runtime_model_dir / module.RUNTIME_METADATA_FILE_NAME
            metadata_path.write_text(
                json.dumps(
                    {
                        "trainingMode": "lora",
                        "baseModelPath": str(temp_root / "missing" / "VoxCPM2"),
                        "baseModelRelativePath": "base-models/VoxCPM2",
                        "latestCheckpointPath": str(temp_root / "missing" / "latest"),
                        "latestCheckpointRelativePath": "checkpoints/lora/latest",
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )

            with patch.object(module, "__file__", str(fake_init_path)), patch.object(
                module, "SRC_MODEL_ROOT", temp_root / "src-model"
            ):
                runtime_target = module.resolve_runtime_target(str(runtime_model_dir))

        self.assertEqual(runtime_target.model_path, str(base_model_dir.resolve()))
        self.assertEqual(
            runtime_target.load_kwargs["lora_weights_path"],
            str(checkpoint_dir.resolve()),
        )

    def test_load_model_and_dependencies_uses_resolved_lora_config(self):
        module = load_runtime_module()

        captured: dict[str, object] = {}

        class FakeLoRAConfig:
            def __init__(self, **kwargs):
                self.kwargs = kwargs

        class FakeVoxCPM:
            @classmethod
            def from_pretrained(cls, model_path, **kwargs):
                captured["model_path"] = model_path
                captured["kwargs"] = kwargs
                return "fake-model"

        fake_deps = SimpleNamespace(sf="fake-sf", VoxCPM=FakeVoxCPM, LoRAConfig=FakeLoRAConfig)
        runtime_target = module.RuntimeTarget(
            model_path="D:/base-models/VoxCPM2",
            load_kwargs={
                "lora_weights_path": "D:/models/13_hare/checkpoints/lora/latest",
                "lora_config_dict": {"r": 16, "alpha": 32},
            },
        )

        with patch.object(module, "load_dependencies", return_value=fake_deps), patch.object(
            module, "resolve_runtime_target", return_value=runtime_target
        ):
            model, deps = module.load_model_and_dependencies("D:/models/13_hare", "cuda:0")

        self.assertEqual(model, "fake-model")
        self.assertIs(deps, fake_deps)
        self.assertEqual(captured["model_path"], "D:/base-models/VoxCPM2")
        self.assertEqual(
            captured["kwargs"]["lora_weights_path"],
            "D:/models/13_hare/checkpoints/lora/latest",
        )
        self.assertEqual(captured["kwargs"]["lora_config"].kwargs["r"], 16)
        self.assertEqual(captured["kwargs"]["lora_config"].kwargs["alpha"], 32)
        self.assertNotIn("lora_config_dict", captured["kwargs"])

    def test_resolve_runtime_target_falls_back_to_default_base_model_for_legacy_metadata(self):
        module = load_runtime_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_init_path = temp_root / "src-model" / "vox_cpm2" / "__init__.py"
            fake_init_path.parent.mkdir(parents=True, exist_ok=True)
            fake_init_path.write_text("# stub", encoding="utf-8")

            base_model_dir = temp_root / "src-model" / "base-models" / "VoxCPM2"
            base_model_dir.mkdir(parents=True, exist_ok=True)
            runtime_model_dir = temp_root / "models" / "legacy_hare"
            checkpoint_dir = runtime_model_dir / "checkpoints" / "lora" / "step_84"
            checkpoint_dir.mkdir(parents=True, exist_ok=True)
            metadata_path = runtime_model_dir / module.RUNTIME_METADATA_FILE_NAME
            metadata_path.parent.mkdir(parents=True, exist_ok=True)
            metadata_path.write_text(
                json.dumps(
                    {
                        "trainingMode": "lora",
                        "baseModelPath": str(temp_root / "missing" / "VoxCPM2"),
                        "latestCheckpointPath": str(temp_root / "missing" / "step_84"),
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )

            with patch.object(module, "__file__", str(fake_init_path)), patch.object(
                module, "SRC_MODEL_ROOT", temp_root / "src-model"
            ):
                runtime_target = module.resolve_runtime_target(str(runtime_model_dir))

        self.assertEqual(runtime_target.model_path, str(base_model_dir.resolve()))
        self.assertEqual(
            runtime_target.load_kwargs["lora_weights_path"],
            str(checkpoint_dir.resolve()),
        )


if __name__ == "__main__":
    unittest.main()