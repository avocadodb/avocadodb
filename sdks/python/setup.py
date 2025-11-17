"""
AvocadoDB Python SDK setup
"""

from setuptools import setup, find_packages
import os

# Read README if it exists
readme_path = os.path.join(os.path.dirname(__file__), "README.md")
long_description = ""
if os.path.exists(readme_path):
    with open(readme_path, "r", encoding="utf-8") as fh:
        long_description = fh.read()

setup(
    name="avocadodb",
    version="2.0.0",
    author="AvocadoDB Team",
    author_email="hello@avocadodb.com",
    description="Deterministic context database for AI agents - Python SDK",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/avocadodb/avocadodb",
    project_urls={
        "Bug Tracker": "https://github.com/avocadodb/avocadodb/issues",
        "Documentation": "https://github.com/avocadodb/avocadodb#readme",
        "Source Code": "https://github.com/avocadodb/avocadodb",
    },
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: Database",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.28.0",
    ],
    extras_require={
        "llm": [
            "transformers>=4.35.0",
            "torch>=2.0.0",
        ],
        "dev": [
            "pytest>=7.0",
            "pytest-cov>=4.0",
            "black>=23.0",
            "mypy>=1.0",
        ],
    },
    keywords="rag retrieval context database ai llm deterministic",
)
