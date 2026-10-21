//! BGE-Small-Zh 嵌入模型
//! 使用 rust_tokenizers (纯 Rust) 避免 Windows CRT 链接冲突

use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use rust_tokenizers::tokenizer::{BertTokenizer, Tokenizer, TruncationStrategy};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum EmbedderError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    #[error("Tokenizer loading failed: {0}")]
    TokenizerLoad(String),
    #[error("Inference failed: {0}")]
    Inference(String),
}

pub struct OnnxEmbedder {
    session: Session,
    tokenizer: BertTokenizer,
    hidden_size: usize,
}

impl OnnxEmbedder {
    /// 从模型目录加载 BGE 嵌入模型
    /// 需要 model.onnx 和 vocab.txt 文件
    pub fn new(model_dir: &Path) -> Result<Self, EmbedderError> {
        let model_path = model_dir.join("model.onnx");
        let vocab_path = model_dir.join("vocab.txt");

        // 加载 ONNX 模型
        let session = Session::builder()
            .map_err(|e| EmbedderError::ModelLoad(e.to_string()))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| EmbedderError::ModelLoad(e.to_string()))?
            .commit_from_file(&model_path)
            .map_err(|e| EmbedderError::ModelLoad(format!("Failed to load model: {}", e)))?;

        // 加载 BERT tokenizer (BGE 使用 WordPiece)
        // 参数: vocab_path, do_lower_case, strip_accents
        let tokenizer = BertTokenizer::from_file(&vocab_path, true, true)
            .map_err(|e| EmbedderError::TokenizerLoad(format!("Failed to load vocab: {:?}", e)))?;

        // BGE-small-zh 的隐藏层大小是 512
        let hidden_size = 512;

        Ok(Self {
            session,
            tokenizer,
            hidden_size,
        })
    }

    /// 获取嵌入向量维度
    pub fn dimension(&self) -> usize {
        self.hidden_size
    }

    /// 生成文本的嵌入向量
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        // 分词 - BGE 模型最大序列长度 512
        let max_len = 512;
        let encoding = self.tokenizer.encode(
            text,
            None,                              // 无第二个句子
            max_len,                           // 最大长度
            &TruncationStrategy::LongestFirst, // 截断策略
            0,                                 // stride (用于长文本分块)
        );

        let input_ids: Vec<i64> = encoding.token_ids;
        let attention_mask: Vec<i64> = vec![1i64; input_ids.len()];
        let token_type_ids: Vec<i64> = vec![0i64; input_ids.len()];

        let seq_len = input_ids.len();

        // 使用 (shape, data) 元组格式创建张量 - ort 2.0 API
        let shape = vec![1usize, seq_len];
        
        let input_ids_tensor = Tensor::from_array((shape.clone(), input_ids.into_boxed_slice()))
            .map_err(|e| EmbedderError::Inference(format!("Failed to create input_ids tensor: {}", e)))?;
        let attention_mask_tensor = Tensor::from_array((shape.clone(), attention_mask.into_boxed_slice()))
            .map_err(|e| EmbedderError::Inference(format!("Failed to create attention_mask tensor: {}", e)))?;
        let token_type_ids_tensor = Tensor::from_array((shape, token_type_ids.into_boxed_slice()))
            .map_err(|e| EmbedderError::Inference(format!("Failed to create token_type_ids tensor: {}", e)))?;

        // 运行推理并提取数据（在 outputs 被释放前复制数据）
        let (output_data, hidden_size) = {
            let outputs = self
                .session
                .run(ort::inputs![
                    "input_ids" => input_ids_tensor,
                    "attention_mask" => attention_mask_tensor,
                    "token_type_ids" => token_type_ids_tensor,
                ])
                .map_err(|e| EmbedderError::Inference(format!("ONNX inference failed: {}", e)))?;

            // 获取输出 - BGE 模型输出 last_hidden_state [batch, seq_len, hidden_size]
            let output_value = outputs
                .get("last_hidden_state")
                .or_else(|| outputs.get("output"))
                .ok_or_else(|| EmbedderError::Inference("No output tensor found".to_string()))?;

            // 提取输出数据 - 返回 (shape, data) 元组
            let (output_shape, data) = output_value
                .try_extract_tensor::<f32>()
                .map_err(|e| EmbedderError::Inference(format!("Failed to extract tensor: {}", e)))?;

            // 获取维度信息
            let shape_dims: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();
            let hidden_size = if shape_dims.len() >= 3 {
                shape_dims[2]
            } else {
                self.hidden_size
            };

            // 复制数据，这样 outputs 可以被释放
            (data.to_vec(), hidden_size)
        };

        // Mean pooling: 对序列维度取平均得到句子向量
        let embedding = Self::mean_pooling(&output_data, seq_len, hidden_size);

        // L2 归一化
        let normalized = Self::l2_normalize(&embedding);

        Ok(normalized)
    }

    /// Mean pooling - 对所有 token 的隐藏状态取平均（静态方法）
    fn mean_pooling(data: &[f32], seq_len: usize, hidden_size: usize) -> Vec<f32> {
        // data 形状是 [1, seq_len, hidden_size]，展平后是连续的
        // 我们需要对 seq_len 维度取平均
        let mut result = vec![0.0f32; hidden_size];
        
        for token_idx in 0..seq_len {
            for hidden_idx in 0..hidden_size {
                let idx = token_idx * hidden_size + hidden_idx;
                if idx < data.len() {
                    result[hidden_idx] += data[idx];
                }
            }
        }
        
        // 取平均
        for val in &mut result {
            *val /= seq_len as f32;
        }
        
        result
    }

    /// L2 归一化（静态方法）
    fn l2_normalize(vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter().map(|x| x / norm).collect()
        } else {
            vec.to_vec()
        }
    }
}
