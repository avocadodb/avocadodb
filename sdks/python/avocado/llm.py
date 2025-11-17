"""
Optional LLM integration for AvocadoDB.

Provides TinyLlama helper for generating natural language answers from context.
This module is optional - AvocadoDB works fine without it.
"""

from typing import Optional, Union
import warnings


class TinyLlamaHelper:
    """Helper for using TinyLlama with AvocadoDB context."""
    
    def __init__(self, device: int = -1, model_name: str = "TinyLlama/TinyLlama-1.1B-Chat-v1.0"):
        """Initialize TinyLlama helper.
        
        Args:
            device: Device ID (-1 for CPU, 0+ for GPU)
            model_name: Model name to load
            
        Raises:
            ImportError: If transformers/torch not installed
        """
        self.device = device
        self.model_name = model_name
        self._model = None
        self._tokenizer = None
    
    def _load_model(self):
        """Lazy-load the model (only when needed)."""
        if self._model is not None:
            return
        
        try:
            from transformers import pipeline
            import torch
        except ImportError:
            raise ImportError(
                "transformers and torch are required for LLM support. "
                "Install with: pip install avocadodb[llm]"
            )
        
        try:
            self._model = pipeline(
                "text-generation",
                model=self.model_name,
                device=self.device,
            )
        except Exception as e:
            raise RuntimeError(f"Failed to load TinyLlama model: {e}")
    
    def generate_answer(
        self,
        query: str,
        context_text: str,
        max_new_tokens: int = 150,
        deterministic: bool = True,
    ) -> str:
        """Generate answer from context.
        
        Args:
            query: The question being asked
            context_text: Context from AvocadoDB compilation
            max_new_tokens: Maximum tokens to generate
            deterministic: Use deterministic generation (no sampling)
            
        Returns:
            Generated answer as string
        """
        self._load_model()
        
        # Format prompt
        prompt = f"""Based on this code:

{context_text[:1500]}

Question: {query}

Answer:"""
        
        # Generate answer
        try:
            output = self._model(
                prompt,
                max_new_tokens=max_new_tokens,
                do_sample=not deterministic,
                pad_token_id=self._model.tokenizer.eos_token_id,
                return_full_text=False,
            )
            
            # Extract answer
            if isinstance(output, list) and len(output) > 0:
                if 'generated_text' in output[0]:
                    answer = output[0]['generated_text']
                    # Remove prompt if included
                    if prompt in answer:
                        answer = answer[len(prompt):].strip()
                    else:
                        answer = answer.strip()
                else:
                    answer = str(output[0])
            else:
                answer = str(output)
            
            return answer
            
        except Exception as e:
            raise RuntimeError(f"Failed to generate answer: {e}")
    
    def is_available(self) -> bool:
        """Check if TinyLlama is available (dependencies installed)."""
        try:
            from transformers import pipeline
            import torch
            return True
        except ImportError:
            return False


# Global singleton instance for LLM (loaded once, reused across calls)
_global_llm_helper: Optional[TinyLlamaHelper] = None


def generate_answer(
    query: str,
    context_text: str,
    model_name: str = "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
    device: int = -1,
    max_new_tokens: int = 150,
    deterministic: bool = True,
) -> str:
    """Convenience function to generate answer from context.
    
    Uses a global singleton instance to avoid reloading the model on every call.
    This makes subsequent calls much faster (model stays in memory).
    
    Args:
        query: The question being asked
        context_text: Context from AvocadoDB compilation
        model_name: Model name to use
        device: Device ID (-1 for CPU, 0+ for GPU)
        max_new_tokens: Maximum tokens to generate
        deterministic: Use deterministic generation
        
    Returns:
        Generated answer as string
        
    Raises:
        ImportError: If transformers/torch not installed
    """
    global _global_llm_helper
    
    # Reuse global instance if it exists and matches our config
    if _global_llm_helper is None:
        _global_llm_helper = TinyLlamaHelper(device=device, model_name=model_name)
    elif _global_llm_helper.model_name != model_name or _global_llm_helper.device != device:
        # Config changed, create new instance
        _global_llm_helper = TinyLlamaHelper(device=device, model_name=model_name)
    
    return _global_llm_helper.generate_answer(
        query=query,
        context_text=context_text,
        max_new_tokens=max_new_tokens,
        deterministic=deterministic,
    )

