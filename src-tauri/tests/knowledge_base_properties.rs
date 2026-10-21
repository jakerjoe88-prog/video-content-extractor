// Property-Based Tests for Knowledge Base
// Feature: tauri-refactor, Property 11: 知识库检索相关性
// Validates: Requirements 17.1.3, 17.1.7

use douyin_creator_tools_lib::ai::knowledge_base::KnowledgeBase;
use proptest::prelude::*;
use tempfile::tempdir;

/// Property 11: 知识库检索相关性
/// For any document uploaded to the knowledge base, searching using keywords from the document
/// should return that document as one of the results.
#[test]
fn test_knowledge_base_search_relevance() {
    proptest!(|(
        content in "[a-zA-Z0-9\\s]{50,200}",
        category in "[a-z]{3,10}"
    )| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            // Create temporary directory and database
            let dir = tempdir().unwrap();
            let db_path = dir.path().join("test_kb.db");
            let kb = KnowledgeBase::new(&db_path).unwrap();

            // Create a temporary text file with the content
            let file_path = dir.path().join("test_doc.txt");
            std::fs::write(&file_path, &content).unwrap();

            // Add document to knowledge base
            let doc = kb.add_document(&file_path, &category).await.unwrap();

            // Extract a keyword from the content (first word with more than 3 characters)
            let keyword = content
                .split_whitespace()
                .find(|word| word.len() > 3)
                .unwrap_or_else(|| {
                    // If no word is long enough, take first 5 chars safely
                    let chars: Vec<char> = content.chars().collect();
                    let len = std::cmp::min(5, chars.len());
                    &content[..content.char_indices().nth(len).map(|(i, _)| i).unwrap_or(content.len())]
                });

            // Search for the keyword
            let results = kb.search(keyword, 10).await.unwrap();

            // Property: The document should be in the search results
            let found = results.iter().any(|result| result.document.id == doc.id);
            prop_assert!(
                found,
                "Document with ID {} should be found when searching for keyword '{}', but it wasn't in results",
                doc.id,
                keyword
            );
            
            Ok(())
        }).unwrap()
    });
}

/// Property: Round-trip consistency
/// For any document added to the knowledge base, it should be retrievable by listing documents
#[test]
fn test_knowledge_base_round_trip() {
    proptest!(|(
        content in "[a-zA-Z0-9\\s]{20,100}",
        category in "[a-z]{3,10}"
    )| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let dir = tempdir().unwrap();
            let db_path = dir.path().join("test_kb.db");
            let kb = KnowledgeBase::new(&db_path).unwrap();

            let file_path = dir.path().join("test_doc.txt");
            std::fs::write(&file_path, &content).unwrap();

            // Add document
            let doc = kb.add_document(&file_path, &category).await.unwrap();

            // List all documents
            let documents = kb.list_documents(None).await.unwrap();

            // Property: The added document should be in the list
            let found = documents.iter().any(|d| d.id == doc.id);
            prop_assert!(
                found,
                "Document with ID {} should be in the list after adding",
                doc.id
            );

            // Property: The content should match
            let retrieved_doc = documents.iter().find(|d| d.id == doc.id).unwrap();
            prop_assert_eq!(&retrieved_doc.content, &content);
            
            Ok(())
        }).unwrap()
    });
}

/// Property: Category filtering
/// For any document added with a specific category, listing documents by that category
/// should include the document
#[test]
fn test_knowledge_base_category_filtering() {
    proptest!(|(
        content in "[a-zA-Z0-9\\s]{20,100}",
        category in "[a-z]{3,10}"
    )| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let dir = tempdir().unwrap();
            let db_path = dir.path().join("test_kb.db");
            let kb = KnowledgeBase::new(&db_path).unwrap();

            let file_path = dir.path().join("test_doc.txt");
            std::fs::write(&file_path, &content).unwrap();

            // Add document with specific category
            let doc = kb.add_document(&file_path, &category).await.unwrap();

            // List documents by category
            let documents = kb.list_documents(Some(&category)).await.unwrap();

            // Property: The document should be in the filtered list
            let found = documents.iter().any(|d| d.id == doc.id);
            prop_assert!(
                found,
                "Document with ID {} and category '{}' should be in the filtered list",
                doc.id,
                category
            );

            // Property: All documents in the list should have the same category
            for d in &documents {
                prop_assert_eq!(&d.category, &category);
            }
            
            Ok(())
        }).unwrap()
    });
}

/// Property: Delete removes document
/// For any document added to the knowledge base, deleting it should remove it from the list
#[test]
fn test_knowledge_base_delete_removes_document() {
    proptest!(|(
        content in "[a-zA-Z0-9\\s]{20,100}",
        category in "[a-z]{3,10}"
    )| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let dir = tempdir().unwrap();
            let db_path = dir.path().join("test_kb.db");
            let kb = KnowledgeBase::new(&db_path).unwrap();

            let file_path = dir.path().join("test_doc.txt");
            std::fs::write(&file_path, &content).unwrap();

            // Add document
            let doc = kb.add_document(&file_path, &category).await.unwrap();

            // Delete document
            kb.delete_document(&doc.id).await.unwrap();

            // List all documents
            let documents = kb.list_documents(None).await.unwrap();

            // Property: The deleted document should NOT be in the list
            let found = documents.iter().any(|d| d.id == doc.id);
            prop_assert!(
                !found,
                "Document with ID {} should NOT be in the list after deletion",
                doc.id
            );
            
            Ok(())
        }).unwrap()
    });
}
