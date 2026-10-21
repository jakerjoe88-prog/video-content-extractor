// 知识库管理

use crate::ai::onnx_embedder::{EmbedderError, OnnxEmbedder};
use crate::ai::vector_db::{SimpleEmbedder, VectorDb, VectorDbError};
use chrono::Utc;
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum KbError {
    #[error("文档不存在: {0}")]
    DocumentNotFound(String),
    #[error("文档解析失败: {0}")]
    ParseFailed(String),
    #[error("数据库错误: {0}")]
    DatabaseError(String),
    #[error("向量化失败: {0}")]
    EmbeddingFailed(String),
    #[error("嵌入模型未安装")]
    EmbedderNotInstalled,
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("向量数据库错误: {0}")]
    VectorDbError(#[from] VectorDbError),
    #[error("ONNX 嵌入错误: {0}")]
    OnnxEmbedderError(#[from] EmbedderError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: String,
    pub name: String,
    pub category: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub relevance: f32,
    pub snippet: String,
}

/// 嵌入器类型（支持回退）
/// 使用 Mutex 包装 OnnxEmbedder 以支持内部可变性（Session::run 需要 &mut self）
pub enum Embedder {
    /// ONNX 语义嵌入（推荐）- 使用 Mutex 支持内部可变性
    Onnx(Mutex<OnnxEmbedder>),
    /// 简单嵌入（回退，效果差）
    Simple(SimpleEmbedder),
}

impl Embedder {
    /// 生成文本嵌入向量
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, KbError> {
        match self {
            Embedder::Onnx(e) => e.lock().embed(text).map_err(KbError::from),
            Embedder::Simple(e) => Ok(e.embed(text)),
        }
    }

    /// 获取向量维度
    pub fn dimension(&self) -> usize {
        match self {
            Embedder::Onnx(e) => e.lock().dimension(),
            Embedder::Simple(e) => e.dimension(),
        }
    }

    /// 是否使用语义嵌入
    pub fn is_semantic(&self) -> bool {
        matches!(self, Embedder::Onnx(_))
    }
}

pub struct KnowledgeBase {
    vector_db: Arc<VectorDb>,
    embedder: Arc<Embedder>,
    documents: Arc<parking_lot::RwLock<Vec<Document>>>,
}

impl Clone for KnowledgeBase {
    fn clone(&self) -> Self {
        Self {
            vector_db: Arc::clone(&self.vector_db),
            embedder: Arc::clone(&self.embedder),
            documents: Arc::clone(&self.documents),
        }
    }
}

impl KnowledgeBase {
    /// 创建新的知识库实例
    ///
    /// # 参数
    /// - `db_path`: 向量数据库路径
    /// - `model_dir`: ONNX 模型目录（可选，如果为 None 或模型不存在则使用 SimpleEmbedder）
    pub fn new(db_path: &Path) -> Result<Self, KbError> {
        Self::with_model_dir(db_path, None)
    }

    /// 创建知识库实例并指定模型目录
    pub fn with_model_dir(db_path: &Path, model_dir: Option<&Path>) -> Result<Self, KbError> {
        let vector_db = Arc::new(VectorDb::new(db_path)?);
        let documents = Arc::new(parking_lot::RwLock::new(Vec::new()));

        // 尝试加载 ONNX 嵌入器，失败则回退到 SimpleEmbedder
        let embedder = if let Some(dir) = model_dir {
            match OnnxEmbedder::new(dir) {
                Ok(onnx) => {
                    tracing::info!("使用 ONNX 语义嵌入模型: {:?}", dir);
                    Arc::new(Embedder::Onnx(Mutex::new(onnx)))
                }
                Err(e) => {
                    tracing::warn!("ONNX 模型加载失败，回退到 SimpleEmbedder: {}", e);
                    Arc::new(Embedder::Simple(SimpleEmbedder::new(384)))
                }
            }
        } else {
            tracing::info!("未指定模型目录，使用 SimpleEmbedder");
            Arc::new(Embedder::Simple(SimpleEmbedder::new(384)))
        };

        // 从数据库加载已保存的文档
        if let Ok(saved_docs) = vector_db.load_documents() {
            let mut docs = documents.write();
            for (id, name, category, content, created_at) in saved_docs {
                docs.push(Document {
                    id,
                    name,
                    category,
                    content,
                    created_at,
                });
            }
            tracing::info!("从数据库加载了 {} 个文档", docs.len());
        }

        Ok(Self {
            vector_db,
            embedder,
            documents,
        })
    }

    /// 检查是否使用语义嵌入
    pub fn is_using_semantic_embedding(&self) -> bool {
        self.embedder.is_semantic()
    }

    /// 添加文档到知识库
    pub async fn add_document(&self, path: &PathBuf, category: &str) -> Result<Document, KbError> {
        // 检查文件是否存在
        if !path.exists() {
            return Err(KbError::DocumentNotFound(path.display().to_string()));
        }

        // 解析文档内容
        let content = self.parse_document(path)?;

        // 生成文档 ID
        let doc_id = Uuid::new_v4().to_string();

        // 生成向量嵌入
        let embedding = self.embedder.embed(&content)?;

        // 保存向量到数据库
        self.vector_db.insert(&doc_id, &embedding)?;

        // 创建文档对象
        let doc = Document {
            id: doc_id,
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            category: category.to_string(),
            content,
            created_at: Utc::now().to_rfc3339(),
        };

        // 保存文档元数据到数据库（持久化）
        self.vector_db.save_document(&doc.id, &doc.name, &doc.category, &doc.content, &doc.created_at)?;

        // 保存到内存中
        self.documents.write().push(doc.clone());

        Ok(doc)
    }

    /// 解析文档内容
    fn parse_document(&self, path: &PathBuf) -> Result<String, KbError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match extension.as_str() {
            "txt" => self.parse_txt(path),
            "docx" => self.parse_docx(path),
            "doc" => Err(KbError::ParseFailed(
                "不支持旧版 .doc 格式，请转换为 .docx".to_string(),
            )),
            "pdf" => self.parse_pdf(path),
            _ => Err(KbError::ParseFailed(format!(
                "不支持的文档格式: {}",
                extension
            ))),
        }
    }

    /// 解析 TXT 文件
    fn parse_txt(&self, path: &Path) -> Result<String, KbError> {
        std::fs::read_to_string(path)
            .map_err(|e| KbError::ParseFailed(format!("读取 TXT 文件失败: {}", e)))
    }

    /// 解析 Word 文档 (.docx)
    fn parse_docx(&self, path: &Path) -> Result<String, KbError> {
        use std::fs::File;
        use std::io::Read;
        use zip::ZipArchive;

        // 打开 docx 文件（实际上是 ZIP 压缩包）
        let file = File::open(path)
            .map_err(|e| KbError::ParseFailed(format!("打开 DOCX 文件失败: {}", e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| KbError::ParseFailed(format!("解析 DOCX 压缩包失败: {}", e)))?;

        // 读取 word/document.xml 文件
        let mut document_xml = archive
            .by_name("word/document.xml")
            .map_err(|e| KbError::ParseFailed(format!("找不到 document.xml: {}", e)))?;

        let mut xml_content = String::new();
        document_xml
            .read_to_string(&mut xml_content)
            .map_err(|e| KbError::ParseFailed(format!("读取 document.xml 失败: {}", e)))?;

        // 简单的 XML 文本提取（提取 <w:t> 标签中的文本）
        let text = self.extract_text_from_xml(&xml_content);

        if text.trim().is_empty() {
            return Err(KbError::ParseFailed("文档内容为空".to_string()));
        }

        Ok(text)
    }

    /// 从 Word XML 中提取文本
    fn extract_text_from_xml(&self, xml: &str) -> String {
        let mut result = String::new();
        let mut pos = 0;
        
        while pos < xml.len() {
            // 查找 <w:t 开始标签（可能是 <w:t> 或 <w:t xml:space="preserve">）
            if let Some(tag_start) = xml[pos..].find("<w:t") {
                let abs_start = pos + tag_start;
                
                // 找到 > 结束
                if let Some(content_start) = xml[abs_start..].find('>') {
                    let abs_content_start = abs_start + content_start + 1;
                    
                    // 找到 </w:t> 结束标签
                    if let Some(content_end) = xml[abs_content_start..].find("</w:t>") {
                        let abs_content_end = abs_content_start + content_end;
                        let text = &xml[abs_content_start..abs_content_end];
                        result.push_str(text);
                        pos = abs_content_end + 6; // 跳过 </w:t>
                        continue;
                    }
                }
                pos = abs_start + 4;
            } else {
                break;
            }
        }
        
        // 处理段落：在句号等标点后添加换行使文本更易读
        let paragraph_count = xml.matches("</w:p>").count();
        if paragraph_count > 1 && !result.is_empty() {
            let mut formatted = String::new();
            let chars: Vec<char> = result.chars().collect();
            for (i, c) in chars.iter().enumerate() {
                formatted.push(*c);
                // 在句号、问号、感叹号后添加换行
                if (*c == '。' || *c == '？' || *c == '！') && i + 1 < chars.len() {
                    formatted.push('\n');
                }
            }
            return formatted;
        }

        result
    }

    /// 解析 PDF 文档
    fn parse_pdf(&self, path: &Path) -> Result<String, KbError> {
        // 使用 pdf-extract 提取文本
        let text = pdf_extract::extract_text(path)
            .map_err(|e| KbError::ParseFailed(format!("解析 PDF 失败: {}", e)))?;

        if text.trim().is_empty() {
            return Err(KbError::ParseFailed("PDF 文档内容为空".to_string()));
        }

        Ok(text)
    }

    /// 搜索相关知识
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, KbError> {
        // 生成查询向量
        let query_embedding = self.embedder.embed(query)?;

        // 搜索相似向量
        let similar_docs = self.vector_db.search(&query_embedding, limit)?;

        // 构建搜索结果
        let documents = self.documents.read();
        let mut results = Vec::new();

        for (doc_id, relevance) in similar_docs {
            if let Some(doc) = documents.iter().find(|d| d.id == doc_id) {
                // 生成摘要片段（取前 200 个字符，确保在字符边界）
                let snippet = if doc.content.len() > 200 {
                    let char_boundary = doc
                        .content
                        .char_indices()
                        .nth(200)
                        .map(|(i, _)| i)
                        .unwrap_or(doc.content.len());
                    format!("{}...", &doc.content[..char_boundary])
                } else {
                    doc.content.clone()
                };

                results.push(SearchResult {
                    document: doc.clone(),
                    relevance,
                    snippet,
                });
            }
        }

        Ok(results)
    }

    /// 删除文档
    pub async fn delete_document(&self, id: &str) -> Result<(), KbError> {
        // 从向量数据库删除
        self.vector_db.delete(id)?;

        // 从文档表删除
        self.vector_db.delete_document(id)?;

        // 从内存中删除
        self.documents.write().retain(|d| d.id != id);

        Ok(())
    }

    /// 清除所有知识库数据
    pub async fn clear_all(&self) -> Result<(), KbError> {
        // 清除向量数据库
        self.vector_db.clear_all()?;

        // 清除内存中的文档
        self.documents.write().clear();

        Ok(())
    }

    /// 获取所有文档
    pub async fn list_documents(&self, category: Option<&str>) -> Result<Vec<Document>, KbError> {
        let documents = self.documents.read();

        if let Some(cat) = category {
            Ok(documents
                .iter()
                .filter(|d| d.category == cat)
                .cloned()
                .collect())
        } else {
            Ok(documents.clone())
        }
    }

    /// 导出知识库
    pub async fn export(&self, path: &PathBuf) -> Result<(), KbError> {
        let documents = self.documents.read();
        let json = serde_json::to_string_pretty(&*documents)
            .map_err(|e| KbError::DatabaseError(format!("序列化失败: {}", e)))?;

        std::fs::write(path, json)?;

        Ok(())
    }

    /// 导入知识库
    pub async fn import(&self, path: &PathBuf) -> Result<(), KbError> {
        let json = std::fs::read_to_string(path)?;
        let imported_docs: Vec<Document> = serde_json::from_str(&json)
            .map_err(|e| KbError::DatabaseError(format!("反序列化失败: {}", e)))?;

        // 重新生成向量并导入
        for doc in imported_docs {
            let embedding = self.embedder.embed(&doc.content)?;
            self.vector_db.insert(&doc.id, &embedding)?;
            self.vector_db
                .save_document(&doc.id, &doc.name, &doc.category, &doc.content, &doc.created_at)?;

            let mut documents = self.documents.write();
            if let Some(existing) = documents.iter_mut().find(|existing| existing.id == doc.id) {
                *existing = doc.clone();
            } else {
                documents.push(doc.clone());
            }
        }

        Ok(())
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        // 使用内存数据库作为默认
        Self::new(&PathBuf::from(":memory:")).unwrap()
    }
}
