#!/usr/bin/env python3
"""
Google Colab GPU Evaluation Script

This creates a Colab notebook that you can run to execute the evaluation on a free GPU.

Usage:
    1. Run this script to generate the Colab notebook
    2. Upload the notebook to Google Colab
    3. Run all cells (gets free GPU automatically)
    4. Download results

This is the simplest option - no API keys, no setup, just free GPU!
"""

from pathlib import Path

# Read the evaluation script
script_dir = Path(__file__).parent
evaluation_script = script_dir / "week2_evaluation.py"

if not evaluation_script.exists():
    print(f"❌ Evaluation script not found: {evaluation_script}")
    exit(1)

eval_code = evaluation_script.read_text()

# Create Colab notebook
notebook = {
    "cells": [
        {
            "cell_type": "markdown",
            "metadata": {},
            "source": [
                "# AvocadoDB Week 2 Evaluation\n",
                "\n",
                "This notebook runs the Week 2 evaluation on a free GPU.\n",
                "\n",
                "**Note**: You need to expose your local AvocadoDB server using ngrok."
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Install dependencies\n",
                "!pip install transformers torch requests\n",
                "!pip install -q git+https://github.com/avocadodb/avocadodb.git#subdirectory=sdks/python || echo 'SDK install failed, will use local copy'"
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Set AvocadoDB URL (replace with your ngrok URL)\n",
                "import os\n",
                "os.environ['AVOCADODB_URL'] = 'https://your-ngrok-url.ngrok.io'  # Replace this!\n",
                "\n",
                "# Verify connection\n",
                "import requests\n",
                "try:\n",
                "    response = requests.get(f\"{os.environ['AVOCADODB_URL']}/health\", timeout=5)\n",
                "    print(f\"✅ Connected to AvocadoDB: {response.status_code}\")\n",
                "except Exception as e:\n",
                "    print(f\"❌ Cannot connect: {e}\")\n",
                "    print(\"\\nSetup ngrok on your local machine:\")\n",
                "    print(\"  1. Install: brew install ngrok/ngrok/ngrok\")\n",
                "    print(\"  2. Run: ngrok http 8765\")\n",
                "    print(\"  3. Copy the HTTPS URL and update AVOCADODB_URL above\")"
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Upload evaluation script\n",
                "# (You can also paste the script content directly)\n",
                "from google.colab import files\n",
                "uploaded = files.upload()\n",
                "\n",
                "# Or create it directly:\n",
                "with open('week2_evaluation.py', 'w') as f:\n",
                "    f.write(r'''" + eval_code.replace("'''", "''' + \"'''\" + r'''") + "''')"
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Check GPU availability\n",
                "import torch\n",
                "print(f\"CUDA available: {torch.cuda.is_available()}\")\n",
                "if torch.cuda.is_available():\n",
                "    print(f\"GPU: {torch.cuda.get_device_name(0)}\")\n",
                "    print(f\"GPU Memory: {torch.cuda.get_device_properties(0).total_memory / 1e9:.1f} GB\")"
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Run evaluation\n",
                "!python week2_evaluation.py"
            ]
        },
        {
            "cell_type": "code",
            "execution_count": None,
            "metadata": {},
            "source": [
                "# Download results\n",
                "from google.colab import files\n",
                "files.download('week2_results.json')\n",
                "files.download('test_set.json')"
            ]
        }
    ],
    "metadata": {
        "colab": {
            "provenance": [],
            "gpuType": "T4"
        },
        "kernelspec": {
            "name": "python3",
            "display_name": "Python 3"
        },
        "language_info": {
            "name": "python"
        },
        "accelerator": "GPU"
    },
    "nbformat": 4,
    "nbformat_minor": 0
}

# Save notebook
notebook_path = script_dir / "week2_evaluation_colab.ipynb"
import json
with open(notebook_path, 'w') as f:
    json.dump(notebook, f, indent=2)

print(f"✅ Colab notebook created: {notebook_path}")
print()
print("Next steps:")
print("  1. Go to https://colab.research.google.com/")
print("  2. Upload the notebook: week2_evaluation_colab.ipynb")
print("  3. Setup ngrok on your local machine:")
print("     brew install ngrok/ngrok/ngrok")
print("     ngrok http 8765")
print("  4. Update AVOCADODB_URL in the notebook with your ngrok URL")
print("  5. Run all cells (Runtime → Run all)")
print("  6. Download results when complete")

