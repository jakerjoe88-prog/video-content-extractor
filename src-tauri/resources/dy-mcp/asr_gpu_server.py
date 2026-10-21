"""
GPU 加速语音识别服务
使用 sherpa-onnx Python 绑定 + SenseVoice 模型
支持 DirectML GPU 加速 (Windows)
"""

import os
import threading
import sys
import time
import wave
import struct
from pathlib import Path
from typing import Optional
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.responses import StreamingResponse
from pydantic import BaseModel
import json
import math

app = FastAPI(title="ASR GPU Server")

# 全局模型实例
recognizer = None
current_device = "cpu"
current_num_threads = 4
model_loaded = False

class TranscribeResponse(BaseModel):
    status: str
    text: Optional[str] = None
    duration_ms: Optional[int] = None
    device: Optional[str] = None
    rtf: Optional[float] = None
    error: Optional[str] = None

class StatusResponse(BaseModel):
    status: str
    model_loaded: bool
    device: str
    gpu_available: bool
    gpu_name: Optional[str] = None

class LoadRequest(BaseModel):
    use_gpu: bool = True
    gpu_device_id: int = 0

class TranscribeRequest(BaseModel):
    audio_path: str
    use_gpu: bool = True
    num_threads: int = 4
    gpu_device_id: int = 0

def get_models_dir() -> Path:
    """获取模型目录"""
    if "ASR_MODELS_DIR" in os.environ:
        return Path(os.environ["ASR_MODELS_DIR"])
    
    if sys.platform == "win32":
        # 使用 Roaming AppData 目录
        app_data = Path(os.environ.get("APPDATA", ""))
        return app_data / "QiushiKLBilibiliPublic" / "models" / "asr" / "sense-voice"
        return Path.home() / ".local" / "share" / "QiushiKLBilibiliPublic" / "models" / "asr" / "sense-voice"

print(f"[ASR-GPU] Server Starting...", flush=True)
model_dir = get_models_dir()
print(f"[ASR-GPU] Model Dir: {model_dir}", flush=True)

if model_dir.exists():
    print(f"[ASR-GPU] Model Files: {[f.name for f in model_dir.iterdir()]}", flush=True)
else:
    print(f"[ASR-GPU] ERROR: Model directory does not exist!", flush=True)

try:
    import sherpa_onnx
    print(f"[ASR-GPU] Sherpa-ONNX imported successfully version: {sherpa_onnx.__version__}", flush=True)
except ImportError as e:
    print(f"[ASR-GPU] ERROR: Failed to import sherpa_onnx: {e}", flush=True)
except Exception as e:
    print(f"[ASR-GPU] ERROR: Unexpected error importing sherpa_onnx: {e}", flush=True)

def detect_gpu() -> tuple:
    """检测可用的 GPU，优先返回 NVIDIA/AMD 独立显卡"""
    try:
        if sys.platform == "win32":
            import subprocess
            result = subprocess.run(
                ["wmic", "path", "win32_VideoController", "get", "name"],
                capture_output=True, text=True
            )
            lines = [l.strip() for l in result.stdout.split('\n') if l.strip() and l.strip() != "Name"]
            
            if lines:
                # 优先选择 NVIDIA 或 AMD 独立显卡
                for i, gpu in enumerate(lines):
                    if "NVIDIA" in gpu.upper() or "AMD" in gpu.upper() or "RADEON" in gpu.upper():
                        print(f"[ASR-GPU] 检测到独立显卡: {gpu} (索引: {i})")
                        return True, gpu, i
                # 如果没有独立显卡，返回第一个
                return True, lines[0], 0
    except Exception as e:
        print(f"[ASR-GPU] GPU 检测失败: {e}")
    
    return False, None, 0

def stream_decode_wav(audio_path: str, chunk_duration_s: int = 30):
    """
    生成器：流式读取 WAV 文件并返回分块的浮点采样数据
    Yields: (samples, progress, is_last)
    """
    try:
        with wave.open(audio_path, 'rb') as wav:
            sample_rate = wav.getframerate()
            n_frames = wav.getnframes()
            n_channels = wav.getnchannels()
            sample_width = wav.getsampwidth()
            
            print(f"[ASR-GPU] 音频信息: sr={sample_rate}, channels={n_channels}, width={sample_width}, frames={n_frames}")
            
            if sample_width not in [2, 4]:
                raise ValueError(f"不支持的采样位深: {sample_width * 8}-bit (仅支持 16/32-bit)")

            chunk_size = int(sample_rate * chunk_duration_s)
            total_chunks = math.ceil(n_frames / chunk_size)
            
            # 格式化字符串缓存
            fmt_code = 'h' if sample_width == 2 else 'i'
            max_val = 32768.0 if sample_width == 2 else 2147483648.0
            
            processed_frames = 0
            
            while True:
                raw_data = wav.readframes(chunk_size)
                if not raw_data:
                    break
                
                n_current_frames = len(raw_data) // (sample_width * n_channels)
                processed_frames += n_current_frames
                
                # 解包
                fmt = f"<{n_current_frames * n_channels}{fmt_code}"
                samples_raw = struct.unpack(fmt, raw_data)
                
                # 转 float 并处理声道
                if n_channels == 1:
                    samples = [s / max_val for s in samples_raw]
                else:
                    # 双声道转单声道 (Average)
                    # 避免创建巨型中间列表，使用生成器表达式
                    samples = [
                        (samples_raw[i] + samples_raw[i+1]) / 2 / max_val 
                        for i in range(0, len(samples_raw), 2)
                    ]
                
                progress = processed_frames / n_frames
                yield samples, progress, False, sample_rate, int(n_frames / sample_rate * 1000)
            
            # 结束
            yield [], 1.0, True, sample_rate, int(n_frames / sample_rate * 1000)
            
    except Exception as e:
        print(f"[ASR-GPU] 流式读取失败: {e}")
        raise e

def load_model(use_gpu: bool = True, gpu_device_id: int = 0, num_threads: int = 4) -> bool:
    """加载 SenseVoice 模型"""
    global recognizer, current_device, model_loaded, current_num_threads
    
    current_num_threads = num_threads
    
    try:
        import sherpa_onnx
        
        models_dir = get_models_dir()
        print(f"[ASR-GPU] 模型目录: {models_dir}")
        
        # 检查模型文件
        model_file = models_dir / "model.onnx"
        model_int8 = models_dir / "model.int8.onnx"
        tokens_file = models_dir / "tokens.txt"
        
        # 优先使用非量化模型 (GPU 友好)
        if model_file.exists():
            actual_model = str(model_file)
        elif model_int8.exists():
            actual_model = str(model_int8)
        else:
            print(f"[ASR-GPU] 模型文件不存在: {model_file}")
            return False
        
        if not tokens_file.exists():
            print(f"[ASR-GPU] tokens 文件不存在: {tokens_file}")
            return False
        
        # 智能选择 provider：尝试 CUDA，失败回退 CPU
        provider = "cpu"
        current_device = "CPU"
        
        if use_gpu:
            # 检查是否有 NVIDIA GPU
            gpu_available, gpu_name, _ = detect_gpu()
            if gpu_available and "nvidia" in gpu_name.lower():
                print(f"[ASR-GPU] 检测到 NVIDIA GPU: {gpu_name}，尝试 CUDA 加速...")
                try:
                    # 先尝试用 CUDA 加载
                    test_recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                        model=actual_model,
                        tokens=str(tokens_file),
                        num_threads=4,
                        provider="cuda",
                        language="auto",
                        use_itn=True,
                        debug=False,
                    )
                    # 成功了！
                    provider = "cuda"
                    current_device = "GPU (CUDA)"
                    print("[ASR-GPU] ✅ CUDA 加速已启用！")
                    # 直接使用这个 recognizer
                    recognizer = test_recognizer
                except Exception as e:
                    print(f"[ASR-GPU] ⚠️ CUDA 初始化失败: {e}")
                    print("[ASR-GPU] 回退到 CPU 模式")
                    provider = "cpu"
            else:
                print(f"[ASR-GPU] 未检测到 NVIDIA GPU (当前: {gpu_name})，使用 CPU 模式")
        
        print(f"[ASR-GPU] 最终配置: Provider={provider}, Threads={num_threads}")
        
        start_time = time.time()
        
        # 如果还没创建 recognizer（没走 CUDA 分支或 CUDA 失败）
        if provider == "cpu":
            recognizer = sherpa_onnx.OfflineRecognizer.from_sense_voice(
                model=actual_model,
                tokens=str(tokens_file),
                num_threads=num_threads,
                provider=provider,
                language="auto",
                use_itn=True,
                debug=True,
            )
        
        load_time = time.time() - start_time
        print(f"[ASR-GPU] 模型加载完成，耗时: {load_time:.2f}s")
        print(f"[ASR-GPU] 当前设备: {current_device}")
        
        model_loaded = True
        return True
        
    except Exception as e:
        print(f"[ASR-GPU] 模型加载失败: {e}")
        import traceback
        traceback.print_exc()
        current_device = "Error"
        return False

@app.get("/health")
async def health_check():
    """健康检查"""
    return {"status": "ok", "service": "asr-gpu"}

@app.get("/status")
async def get_status() -> StatusResponse:
    """获取服务状态"""
    gpu_available, gpu_name, _ = detect_gpu()
    return StatusResponse(
        status="ok",
        model_loaded=model_loaded,
        device=current_device,
        gpu_available=gpu_available,
        gpu_name=gpu_name
    )

@app.post("/load")
async def load_model_endpoint(request: LoadRequest = None):
    """加载模型"""
    use_gpu = request.use_gpu if request else True
    gpu_device_id = request.gpu_device_id if request else 0
    
    success = load_model(use_gpu=use_gpu, gpu_device_id=gpu_device_id)
    if success:
        return {"status": "ok", "message": "模型加载成功", "device": current_device}
    else:
        raise HTTPException(status_code=500, detail="模型加载失败")

@app.post("/unload")
async def unload_model():
    """卸载模型"""
    global recognizer, model_loaded
    recognizer = None
    model_loaded = False
    return {"status": "ok", "message": "模型已卸载"}

def transcribe_generator(request: TranscribeRequest):
    """生成器：分块转写并在每一块完成后 yield 进度"""
    global recognizer, current_device, model_loaded
    
    start_time = time.time()
    
    # 1. 确保模型已加载
    if not model_loaded:
        yield f"data: {json.dumps({'status': 'loading_model', 'progress': 0.0})}\n\n"
        success = load_model(request.use_gpu, request.gpu_device_id, request.num_threads)
        if not success:
            yield f"data: {json.dumps({'status': 'error', 'error': 'Failed to load model'})}\n\n"
            return

    # 2. 检查文件
    if not os.path.exists(request.audio_path):
        yield f"data: {json.dumps({'status': 'error', 'error': 'Audio file not found'})}\n\n"
        return

    full_text = []
    
    try:
        print(f"[ASR-GPU] 开始流式转写: {request.audio_path}")
        chunk_gen = stream_decode_wav(request.audio_path, chunk_duration_s=30)
        
        for samples, progress, is_last, sample_rate, duration_ms in chunk_gen:
            if is_last:
                break
                
            if not samples:
                continue

            # 每个块创建一个流 (SenseVoice 此处简化处理，理想情况应维护 context)
            stream = recognizer.create_stream()
            stream.accept_waveform(sample_rate, samples)
            recognizer.decode_stream(stream)
            chunk_text = stream.result.text
            
            if chunk_text:
                full_text.append(chunk_text)
            
            # 调整进度显示 (0.01 - 0.99)
            display_progress = min(0.99, max(0.01, progress))
            yield f"data: {json.dumps({'status': 'processing', 'progress': display_progress})}\n\n"
            
            # 手动触发 GC，以防 accumulating garbage
            # import gc; gc.collect() 

        # 4. 汇总
        final_text = "".join(full_text)
        process_duration = (time.time() - start_time) * 1000
        
        # 再次获取时长用于计算 RTF
        total_duration_ms = duration_ms # 从 stream_decode_wav 最后的 yield 获取
        
        rtf = process_duration / total_duration_ms if total_duration_ms > 0 else 0
        
        print(f"[ASR-GPU] 转写完成，RTF: {rtf:.3f}, 时长: {total_duration_ms}ms")
        
        response = {
            "status": "success",
            "text": final_text,
            "duration_ms": int(process_duration),
            "device": current_device,
            "rtf": round(rtf, 3),
            "progress": 1.0
        }
        yield f"data: {json.dumps(response)}\n\n"

    except Exception as e:
        print(f"[ASR-GPU] 转写过程中出错: {e}")
        import traceback
        traceback.print_exc()
        yield f"data: {json.dumps({'status': 'error', 'error': str(e)})}\n\n"


@app.post("/transcribe")
def transcribe(request: TranscribeRequest):
    return StreamingResponse(transcribe_generator(request), media_type="text/event-stream")

# 配置
ASR_PORT = int(os.environ.get("ASR_GPU_PORT", "38191"))
ASR_HOST = os.environ.get("ASR_GPU_HOST", "127.0.0.1")

def _parent_is_alive(pid: int) -> bool:
    if pid <= 0:
        return True
    try:
        import ctypes
        handle = ctypes.windll.kernel32.OpenProcess(0x1000, False, pid)
        if handle:
            ctypes.windll.kernel32.CloseHandle(handle)
            return True
        return False
    except Exception:
        return True

def _watch_parent():
    raw_pid = os.environ.get("SIDECAR_PARENT_PID", "").strip()
    if not raw_pid.isdigit():
        return
    parent_pid = int(raw_pid)
    while True:
        time.sleep(2)
        if not _parent_is_alive(parent_pid):
            os._exit(0)

if __name__ == "__main__":
    threading.Thread(target=_watch_parent, daemon=True).start()
    print("[ASR-GPU] 启动 GPU 语音识别服务...")
    print(f"[ASR-GPU] 服务地址: http://{ASR_HOST}:{ASR_PORT}")
    
    # 启动时自动加载模型 (GPU 模式)
    load_model(use_gpu=True, gpu_device_id=0)
    
    uvicorn.run(app, host=ASR_HOST, port=ASR_PORT)
