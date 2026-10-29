<div align="center"><!-- readme-style:center -->

# SparseFlow: Efficient Diffusion with Learned Sparsity

Official implementation of **SparseFlow** (NeurIPS 2026).

[Paper](https://arxiv.org/abs/2605.01234) | [Project Page](https://acme-lab.github.io/sparseflow) | [Models](https://huggingface.co/acme-lab/sparseflow-xl) | [Demo](https://huggingface.co/spaces/acme-lab/sparseflow) | [Colab](https://colab.research.google.com/github/acme-lab/sparseflow/blob/main/demo.ipynb)

<!-- readme-style:header theme=startup -->
[![Tests](https://img.shields.io/github/actions/workflow/status/acme-lab/sparseflow/tests.yml?label=Tests&logo=github&style=for-the-badge)](https://github.com/acme-lab/sparseflow/actions/workflows/tests.yml) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue?style=for-the-badge)](LICENSE)

**[News](#news)** · **[Installation](#installation)** · **[Quick Start](#quick-start)** · **[Results](#results)** · **[Training](#training)** · **[Citation](#citation)**

---
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

SparseFlow learns which attention blocks a diffusion transformer can skip at each
denoising step, cutting sampling cost by 2.4x with no loss in FID.

## News

- **2026-09-25**: SparseFlow is accepted at NeurIPS 2026.
- **2026-08-30**: Released the XL checkpoint on Hugging Face.
- **2026-08-12**: Added a Colab demo.
- **2026-07-28**: Training code for ImageNet 512 is out.
- **2026-07-03**: Multi-node training with FSDP.
- **2026-06-20**: Code release.
- **2026-05-14**: Paper on arXiv.
- **2026-05-02**: Project page online.
- **2026-04-18**: Internal preview.

## Installation

```bash
conda create -n sparseflow python=3.11 -y
conda activate sparseflow
pip install -e .
```

> [!NOTE]
> sampling needs a GPU with at least 24 GB of memory.

## Quick Start

```python
from sparseflow import SparseFlowPipeline

pipe = SparseFlowPipeline.from_pretrained("acme-lab/sparseflow-xl").to("cuda")
image = pipe("a corgi astronaut, studio lighting", steps=28).images[0]
```

## Results

| Model | Params | FID (ImageNet 512) | Speedup |
|-------|--------|--------------------|---------|
| DiT-XL/2 | 675M | 3.04 | 1.0x |
| SparseFlow-XL | 675M | 2.98 | 2.4x |

## Training

```bash
torchrun --nproc_per_node 8 train.py --config configs/xl_512.yaml
```

> [!TIP]
> pass `--sparsity-warmup 5000` when fine-tuning from a dense checkpoint.

## Citation

```bibtex
@inproceedings{rossi2026sparseflow,
  title     = {SparseFlow: Efficient Diffusion with Learned Sparsity},
  author    = {Rossi, Maria and Chen, Wei and Okafor, David},
  booktitle = {Advances in Neural Information Processing Systems},
  year      = {2026},
  journal   = {arXiv preprint arXiv:2605.01234}
}
```

## Acknowledgements

This code builds on [diffusers](https://github.com/huggingface/diffusers) and [DiT](https://arxiv.org/abs/2212.09748).

## License

Apache 2.0
