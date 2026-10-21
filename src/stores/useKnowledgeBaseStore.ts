import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface Document {
  id: string;
  name: string;
  category: string;
  content: string;
  created_at: string;
}

export interface SearchResult {
  document: Document;
  relevance: number;
  snippet: string;
}

interface KnowledgeBaseState {
  documents: Document[];
  searchResults: SearchResult[];
  isLoading: boolean;
  error: string | null;
  selectedCategory: string | null;

  // Actions
  initKnowledgeBase: (dbPath: string) => Promise<void>;
  addDocument: (filePath: string, category: string) => Promise<void>;
  searchKnowledgeBase: (query: string, limit?: number) => Promise<void>;
  deleteDocument: (documentId: string) => Promise<void>;
  listDocuments: (category?: string) => Promise<void>;
  exportKnowledgeBase: (exportPath: string) => Promise<void>;
  importKnowledgeBase: (importPath: string) => Promise<void>;
  setSelectedCategory: (category: string | null) => void;
  clearError: () => void;
}

export const useKnowledgeBaseStore = create<KnowledgeBaseState>((set, get) => ({
  documents: [],
  searchResults: [],
  isLoading: false,
  error: null,
  selectedCategory: null,

  initKnowledgeBase: async (dbPath: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('init_knowledge_base', { dbPath });
      await get().listDocuments();
    } catch (error) {
      set({ error: String(error) });
    } finally {
      set({ isLoading: false });
    }
  },

  addDocument: async (filePath: string, category: string) => {
    set({ isLoading: true, error: null });
    try {
      const document = await invoke<Document>('add_document_to_kb', {
        filePath,
        category,
      });
      set((state) => ({
        documents: [...state.documents, document],
      }));
    } catch (error) {
      set({ error: String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  searchKnowledgeBase: async (query: string, limit = 10) => {
    set({ isLoading: true, error: null });
    try {
      const results = await invoke<SearchResult[]>('search_knowledge_base', {
        query,
        limit,
      });
      set({ searchResults: results });
    } catch (error) {
      set({ error: String(error) });
    } finally {
      set({ isLoading: false });
    }
  },

  deleteDocument: async (documentId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('delete_document_from_kb', { documentId });
      set((state) => ({
        documents: state.documents.filter((doc) => doc.id !== documentId),
      }));
    } catch (error) {
      set({ error: String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  listDocuments: async (category?: string) => {
    set({ isLoading: true, error: null });
    try {
      const documents = await invoke<Document[]>('list_documents', {
        category: category || null,
      });
      set({ documents });
    } catch (error) {
      set({ error: String(error) });
    } finally {
      set({ isLoading: false });
    }
  },

  exportKnowledgeBase: async (exportPath: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('export_knowledge_base', { exportPath });
    } catch (error) {
      set({ error: String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  importKnowledgeBase: async (importPath: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('import_knowledge_base', { importPath });
      await get().listDocuments();
    } catch (error) {
      set({ error: String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  setSelectedCategory: (category: string | null) => {
    set({ selectedCategory: category });
    get().listDocuments(category || undefined);
  },

  clearError: () => set({ error: null }),
}));
