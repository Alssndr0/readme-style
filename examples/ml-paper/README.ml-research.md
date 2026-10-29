<div align="center"><!-- readme-style:center -->

# SparseFlow: Efficient Diffusion with Learned Sparsity

Official implementation of **SparseFlow** (NeurIPS 2026).

[Paper](https://arxiv.org/abs/2605.01234) | [Project Page](https://acme-lab.github.io/sparseflow) | [Models](https://huggingface.co/acme-lab/sparseflow-xl) | [Demo](https://huggingface.co/spaces/acme-lab/sparseflow) | [Colab](https://colab.research.google.com/github/acme-lab/sparseflow/blob/main/demo.ipynb)

<!-- readme-style:header theme=ml-research -->
[![arXiv 2605.01234](https://img.shields.io/badge/arXiv-2605.01234-B31B1B?logo=arxiv&logoColor=white&style=flat)](https://arxiv.org/abs/2605.01234) [![Project page](https://img.shields.io/badge/Project_Page-2F80ED?style=flat)](https://acme-lab.github.io/sparseflow) [![Hugging Face models](https://img.shields.io/badge/Hugging_Face-Models-FFD21E?logo=huggingface&logoColor=000&style=flat)](https://huggingface.co/acme-lab/sparseflow-xl) [![Hugging Face demo](https://img.shields.io/badge/Hugging_Face-Demo-FFD21E?logo=huggingface&logoColor=000&style=flat)](https://huggingface.co/spaces/acme-lab/sparseflow) [![Open in Colab](https://img.shields.io/badge/Open_in-Colab-F9AB00?logo=googlecolab&logoColor=white&style=flat)](https://colab.research.google.com/github/acme-lab/sparseflow/blob/main/demo.ipynb) ![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-3776AB?logo=python&logoColor=white&style=flat) [![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat)](LICENSE)

**[News](#news)** · **[Installation](#installation)** · **[Quick Start](#quick-start)** · **[Results](#results)** · **[Training](#training)** · **[Citation](#citation)**
<!-- /readme-style:header -->

</div><!-- /readme-style:center -->

SparseFlow learns which attention blocks a diffusion transformer can skip at each
denoising step, cutting sampling cost by 2.4x with no loss in FID.

<!-- readme-style:toc theme=ml-research -->
<details>
<summary><b>Table of Contents</b></summary>

- [News](#news)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Results](#results)
- [Training](#training)
- [Citation](#citation)
- [Acknowledgements](#acknowledgements)
- [License](#license)

</details>
<!-- /readme-style:toc -->

## News

<details><summary>Click to expand</summary><!-- readme-style:collapse -->

- **2026-09-25**: SparseFlow is accepted at NeurIPS 2026.
- **2026-08-30**: Released the XL checkpoint on Hugging Face.
- **2026-08-12**: Added a Colab demo.
- **2026-07-28**: Training code for ImageNet 512 is out.
- **2026-07-03**: Multi-node training with FSDP.
- **2026-06-20**: Code release.
- **2026-05-14**: Paper on arXiv.
- **2026-05-02**: Project page online.
- **2026-04-18**: Internal preview.

</details><!-- /readme-style:collapse -->

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
