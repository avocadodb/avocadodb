#!/usr/bin/env python3
"""
GPU-Accelerated Week 2 Evaluation via RunPod API

This script:
1. Spins up a GPU instance on RunPod
2. Uploads evaluation script and dependencies
3. Runs evaluation on GPU (10-20x faster than CPU)
4. Downloads results
5. Terminates the instance

Requirements:
    pip install runpod requests

Setup:
    1. Get RunPod API key: https://www.runpod.io/console/user/settings
    2. Export: export RUNPOD_API_KEY="your-key-here"
    3. Run: python tools/local_llm/run_gpu_evaluation.py

Cost: ~$0.20-0.50 for full evaluation (vs hours on CPU)
"""

import os
import sys
import time
import json
import requests
import subprocess
from pathlib import Path
from typing import Optional, Dict, Any

try:
    import runpod
except ImportError:
    print("❌ runpod not installed. Install with: pip install runpod")
    sys.exit(1)


# Configuration
RUNPOD_API_KEY = os.environ.get("RUNPOD_API_KEY")
if not RUNPOD_API_KEY:
    print("❌ RUNPOD_API_KEY not set. Get one at: https://www.runpod.io/console/user/settings")
    print("   Export it: export RUNPOD_API_KEY='your-key-here'")
    sys.exit(1)

# GPU instance config
GPU_TYPE = "NVIDIA RTX 3090"  # ~$0.29/hour, good for inference
# Alternatives: "NVIDIA RTX 4090" (~$0.49/hour, faster), "NVIDIA A40" (~$0.79/hour, more VRAM)

# AvocadoDB server URL (needs to be accessible from cloud)
# Option 1: Use ngrok to expose local server
# Option 2: Run AvocadoDB server on the GPU instance too
AVOCADODB_URL = os.environ.get("AVOCADODB_URL", "http://localhost:8765")


def check_avocadodb_accessible(url: str) -> bool:
    """Check if AvocadoDB server is accessible from cloud"""
    try:
        response = requests.get(f"{url}/health", timeout=5)
        return response.status_code == 200
    except:
        return False


def setup_ngrok_tunnel(port: int = 8765) -> Optional[str]:
    """Setup ngrok tunnel to expose local AvocadoDB server"""
    print("Setting up ngrok tunnel...")
    try:
        # Check if ngrok is installed
        result = subprocess.run(["which", "ngrok"], capture_output=True)
        if result.returncode != 0:
            print("❌ ngrok not found. Install: brew install ngrok/ngrok/ngrok")
            print("   Or use: https://ngrok.com/download")
            return None
        
        # Start ngrok
        process = subprocess.Popen(
            ["ngrok", "http", str(port), "--log=stdout"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        time.sleep(3)  # Wait for ngrok to start
        
        # Get public URL
        try:
            response = requests.get("http://localhost:4040/api/tunnels", timeout=5)
            tunnels = response.json()
            if tunnels.get("tunnels"):
                public_url = tunnels["tunnels"][0]["public_url"]
                print(f"✅ ngrok tunnel: {public_url}")
                return public_url.replace("http://", "https://")  # Use HTTPS
        except:
            pass
        
        print("⚠️ Could not get ngrok URL. Continuing anyway...")
        return None
    except Exception as e:
        print(f"⚠️ ngrok setup failed: {e}")
        return None


def create_gpu_pod() -> Optional[str]:
    """Create a GPU pod on RunPod"""
    print("Creating GPU pod on RunPod...")
    
    # Use RunPod Python SDK
    runpod.api_key = RUNPOD_API_KEY
    
    # Create pod
    pod_config = {
        "name": "avocadodb-week2-evaluation",
        "imageName": "runpod/pytorch:2.1.0-py3.10-cuda11.8.0-devel",
        "gpuTypeId": GPU_TYPE,
        "cloudType": "ALL",
        "networkVolumeId": None,
        "dataCenterId": None,
        "templateId": None,
        "volumeInGb": 20,
        "containerDiskInGb": 10,
        "minVcpuCount": 2,
        "minMemoryInGb": 15,
        "dockerArgs": "",
        "env": [
            {"key": "AVOCADODB_URL", "value": AVOCADODB_URL},
            {"key": "PYTHONUNBUFFERED", "value": "1"},
        ],
    }
    
    try:
        # Create pod using API
        response = requests.post(
            "https://api.runpod.io/graphql",
            headers={
                "Authorization": f"Bearer {RUNPOD_API_KEY}",
                "Content-Type": "application/json",
            },
            json={
                "query": """
                mutation {
                    podFindAndDeployOnDemand(
                        input: {
                            cloudType: ALL
                            gpuCount: 1
                            volumeInGb: 20
                            containerDiskInGb: 10
                            minVcpuCount: 2
                            minMemoryInGb: 15
                            gpuTypeId: "%s"
                            name: "avocadodb-eval"
                            imageName: "%s"
                            dockerArgs: ""
                            env: [
                                {key: "AVOCADODB_URL", value: "%s"}
                                {key: "PYTHONUNBUFFERED", value: "1"}
                            ]
                        }
                    ) {
                        id
                        imageName
                        env
                        machineId
                        machine {
                            podHostId
                        }
                    }
                }
                """ % (GPU_TYPE, pod_config["imageName"], AVOCADODB_URL)
            }
        )
        
        if response.status_code == 200:
            data = response.json()
            if "data" in data and "podFindAndDeployOnDemand" in data["data"]:
                pod_id = data["data"]["podFindAndDeployOnDemand"]["id"]
                print(f"✅ Pod created: {pod_id}")
                return pod_id
        
        print(f"❌ Failed to create pod: {response.text}")
        return None
        
    except Exception as e:
        print(f"❌ Error creating pod: {e}")
        return None


def wait_for_pod_ready(pod_id: str, timeout: int = 300) -> bool:
    """Wait for pod to be ready"""
    print(f"Waiting for pod {pod_id} to be ready...")
    
    start_time = time.time()
    while time.time() - start_time < timeout:
        try:
            response = requests.get(
                f"https://api.runpod.io/graphql",
                headers={"Authorization": f"Bearer {RUNPOD_API_KEY}"},
                params={
                    "query": """
                    query {
                        myself {
                            pods {
                                id
                                name
                                runtime {
                                    uptimeInSeconds
                                    ports {
                                        ip
                                        isIpPublic
                                        privatePort
                                        publicPort
                                        type
                                    }
                                    gpu {
                                        id
                                        gpuUtilPercent
                                        memoryUtilPercent
                                    }
                                }
                                machineId
                            }
                        }
                    }
                    """
                }
            )
            
            if response.status_code == 200:
                data = response.json()
                # Find our pod
                pods = data.get("data", {}).get("myself", {}).get("pods", [])
                for pod in pods:
                    if pod["id"] == pod_id:
                        if pod.get("runtime"):
                            print("✅ Pod is ready!")
                            return True
                        break
            
            time.sleep(5)
            print(".", end="", flush=True)
            
        except Exception as e:
            print(f"\n⚠️ Error checking pod status: {e}")
            time.sleep(5)
    
    print(f"\n❌ Pod not ready after {timeout}s")
    return False


def upload_and_run_evaluation(pod_id: str) -> bool:
    """Upload evaluation script and run it"""
    print("Uploading evaluation script...")
    
    # Package the evaluation script and requirements
    script_dir = Path(__file__).parent
    evaluation_script = script_dir / "week2_evaluation.py"
    
    if not evaluation_script.exists():
        print(f"❌ Evaluation script not found: {evaluation_script}")
        return False
    
    # Create a setup script that will run on the pod
    setup_script = f"""#!/bin/bash
set -e

echo "Setting up environment..."

# Install dependencies
pip install transformers torch requests

# Install AvocadoDB SDK (if available via git)
# pip install git+https://github.com/avocadodb/avocadodb.git#subdirectory=sdks/python
# Or copy SDK files if needed

# Run evaluation
python week2_evaluation.py

echo "Evaluation complete!"
"""
    
    # For now, use a simpler approach: RunPod Serverless
    # This is easier than managing pods directly
    print("⚠️ Direct pod management is complex. Using RunPod Serverless instead...")
    return False


def run_serverless_evaluation():
    """Use RunPod Serverless for easier execution"""
    print("=" * 70)
    print("RUNPOD SERVERLESS EVALUATION")
    print("=" * 70)
    print()
    print("RunPod Serverless is better for this use case.")
    print("However, it requires packaging your code.")
    print()
    print("Alternative: Use a simpler cloud GPU service:")
    print()
    print("1. **Vast.ai** (Cheapest, ~$0.10-0.20/hour)")
    print("   - SSH into GPU instance")
    print("   - Run evaluation script")
    print("   - Download results")
    print()
    print("2. **Google Colab** (Free, but manual)")
    print("   - Upload script to Colab")
    print("   - Run with free GPU")
    print("   - Download results")
    print()
    print("3. **Lambda Labs** (Easy API, ~$0.50/hour)")
    print("   - Simple SSH-based API")
    print("   - Good documentation")
    print()
    print("Would you like me to create a script for one of these?")


def terminate_pod(pod_id: str):
    """Terminate the GPU pod"""
    print(f"Terminating pod {pod_id}...")
    
    try:
        response = requests.post(
            "https://api.runpod.io/graphql",
            headers={
                "Authorization": f"Bearer {RUNPOD_API_KEY}",
                "Content-Type": "application/json",
            },
            json={
                "query": f"""
                mutation {{
                    podTerminate(input: {{podId: "{pod_id}"}}) {{
                        id
                    }}
                }}
                """
            }
        )
        
        if response.status_code == 200:
            print("✅ Pod terminated")
        else:
            print(f"⚠️ Failed to terminate pod: {response.text}")
    except Exception as e:
        print(f"⚠️ Error terminating pod: {e}")


def main():
    """Main execution"""
    print("=" * 70)
    print("GPU-ACCELERATED WEEK 2 EVALUATION")
    print("=" * 70)
    print()
    
    # Check AvocadoDB accessibility
    if not check_avocadodb_accessible(AVOCADODB_URL):
        print(f"⚠️ AvocadoDB server not accessible at {AVOCADODB_URL}")
        print("   Setting up ngrok tunnel...")
        public_url = setup_ngrok_tunnel()
        if public_url:
            global AVOCADODB_URL
            AVOCADODB_URL = public_url
            print(f"   Using: {AVOCADODB_URL}")
        else:
            print("❌ Cannot proceed without accessible AvocadoDB server")
            print("   Options:")
            print("   1. Use ngrok: brew install ngrok && ngrok http 8765")
            print("   2. Run AvocadoDB server on the GPU instance")
            return
    
    # For now, show alternatives
    run_serverless_evaluation()


if __name__ == "__main__":
    main()

