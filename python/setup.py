"""
AvocadoDB Python SDK setup
"""

from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read() if fh else ""

setup(
    name="avocadodb",
    version="0.1.0",
    author="AvocadoDB Team",
    author_email="team@avocadodb.com",
    description="Deterministic context compilation for AI agents",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/avocadodb/avocadodb",
    py_modules=["avocado"],
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.28.0",
    ],
)
