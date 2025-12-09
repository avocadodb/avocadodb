import modal
from fastapi import FastAPI
from pydantic import BaseModel
from typing import List, Optional

# Base image with required dependencies for GPU embeddings service
image = modal.Image.debian_slim().pip_install(
    "sentence-transformers==2.6.1",
    "torch",
    "numpy",
    "fastapi",
    "uvicorn",
)

app = modal.App("avocado-embed")
web = FastAPI()


class EmbedReq(BaseModel):
    inputs: List[str]
    model: Optional[str] = "BAAI/bge-large-en-v1.5"


class EmbedResp(BaseModel):
    embeddings: List[List[float]]
    dimension: int


@app.function(image=image, cpu=2, gpu="A10G", timeout=1800, min_containers=0)
@modal.asgi_app()
def fastapi_app():
    from sentence_transformers import SentenceTransformer

    model_cache = {"model": None, "name": None}

    @web.post("/embed", response_model=EmbedResp)
    def embed(req: EmbedReq):
        name = req.model or "BAAI/bge-large-en-v1.5"
        if model_cache["name"] != name:
            # Ensure model is on GPU
            model_cache["model"] = SentenceTransformer(name, device="cuda")
            model_cache["name"] = name
        model = model_cache["model"]
        # Larger batch to better utilize GPU
        vecs = model.encode(req.inputs, normalize_embeddings=True, batch_size=512)
        dim = len(vecs[0]) if len(vecs) > 0 else 0
        return {"embeddings": vecs.tolist(), "dimension": dim}

    return web


