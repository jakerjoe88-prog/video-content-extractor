import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface VideoStats {
  likes: number;
  favorites?: number;
  comments: number;
  shares: number;
}

export interface CopyItem {
  id: string;
  libraryId: string;
  sourcePlatform: string;
  sourceAuthor: string;
  sourceTitle: string;
  sourceUrl: string;
  coverUrl?: string;
  localVideoPath?: string;
  stats: VideoStats;
  rawTranscript: string;
  cleanTranscript: string;
  note: string;
  extractedAt: string;
  createdAt: string;
  updatedAt: string;
}

export interface CopyLibrary {
  id: string;
  name: string;
  sourcePlatform: string;
  sourceAuthor: string;
  authorHomepage?: string;
  tags: string[];
  note: string;
  createdAt: string;
  updatedAt: string;
  itemIds: string[];
}

export interface GeneratedScript {
  id: string;
  title: string;
  content: string;
  referenceLibraryId?: string;
  referenceLibraryName?: string;
  createdAt: string;
  updatedAt: string;
}

export interface VerifiedRecoveryPatch {
  item_id: string;
  library_id: string;
  source_url: string;
  restored_fields: Pick<
    CopyItem,
    "sourcePlatform" | "sourceAuthor" | "sourceTitle" | "rawTranscript" | "cleanTranscript"
  >;
}

interface CopyLibraryState {
  libraries: CopyLibrary[];
  items: CopyItem[];
  scripts: GeneratedScript[];
  selectedLibraryId: string | null;

  createLibrary: (input: {
    name: string;
    sourcePlatform?: string;
    sourceAuthor?: string;
    authorHomepage?: string;
    tags?: string[];
    note?: string;
  }) => string;
  updateLibrary: (id: string, updates: Partial<Omit<CopyLibrary, "id" | "itemIds" | "createdAt">>) => void;
  deleteLibrary: (id: string) => void;
  addItemsToLibrary: (libraryId: string, items: Array<Omit<CopyItem, "id" | "libraryId" | "createdAt" | "updatedAt">>) => void;
  deleteItem: (itemId: string) => void;
  updateItem: (itemId: string, updates: Partial<Omit<CopyItem, "id" | "libraryId" | "createdAt">>) => void;
  applyVerifiedRecoveryPatch: (patches: VerifiedRecoveryPatch[]) => number;
  setSelectedLibraryId: (libraryId: string | null) => void;

  saveScript: (input: {
    title: string;
    content: string;
    referenceLibraryId?: string;
    referenceLibraryName?: string;
  }) => string;
  updateScript: (id: string, updates: Partial<Omit<GeneratedScript, "id" | "createdAt">>) => void;
  deleteScript: (id: string) => void;

  getLibraryItems: (libraryId: string) => CopyItem[];
  getSelectedLibrary: () => CopyLibrary | null;
}

const createId = (prefix: string) =>
  `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const nowIso = () => new Date().toISOString();

const normalizeTags = (tags?: string[]) =>
  (tags || [])
    .map((tag) => tag.trim())
    .filter(Boolean)
    .slice(0, 12);

export const useCopyLibraryStore = create<CopyLibraryState>()(
  persist(
    (set, get) => ({
      libraries: [],
      items: [],
      scripts: [],
      selectedLibraryId: null,

      createLibrary: (input) => {
        const id = createId("library");
        const timestamp = nowIso();
        const library: CopyLibrary = {
          id,
          name: input.name.trim() || "未命名文案库",
          sourcePlatform: input.sourcePlatform || "抖音",
          sourceAuthor: input.sourceAuthor || "",
          authorHomepage: input.authorHomepage,
          tags: normalizeTags(input.tags),
          note: input.note || "",
          createdAt: timestamp,
          updatedAt: timestamp,
          itemIds: [],
        };

        set((state) => ({
          libraries: [library, ...state.libraries],
          selectedLibraryId: id,
        }));

        return id;
      },

      updateLibrary: (id, updates) => {
        set((state) => ({
          libraries: state.libraries.map((library) =>
            library.id === id
              ? {
                  ...library,
                  ...updates,
                  tags: updates.tags ? normalizeTags(updates.tags) : library.tags,
                  updatedAt: nowIso(),
                }
              : library
          ),
        }));
      },

      deleteLibrary: (id) => {
        set((state) => ({
          libraries: state.libraries.filter((library) => library.id !== id),
          items: state.items.filter((item) => item.libraryId !== id),
          selectedLibraryId:
            state.selectedLibraryId === id ? null : state.selectedLibraryId,
        }));
      },

      addItemsToLibrary: (libraryId, newItems) => {
        const timestamp = nowIso();
        const items: CopyItem[] = newItems.map((item) => ({
          ...item,
          id: createId("copy"),
          libraryId,
          createdAt: timestamp,
          updatedAt: timestamp,
        }));

        set((state) => ({
          items: [...items, ...state.items],
          libraries: state.libraries.map((library) =>
            library.id === libraryId
              ? {
                  ...library,
                  itemIds: [...items.map((item) => item.id), ...library.itemIds],
                  updatedAt: timestamp,
                }
              : library
          ),
        }));
      },

      deleteItem: (itemId) => {
        const item = get().items.find((candidate) => candidate.id === itemId);
        if (!item) return;

        set((state) => ({
          items: state.items.filter((candidate) => candidate.id !== itemId),
          libraries: state.libraries.map((library) =>
            library.id === item.libraryId
              ? {
                  ...library,
                  itemIds: library.itemIds.filter((id) => id !== itemId),
                  updatedAt: nowIso(),
                }
              : library
          ),
        }));
      },

      updateItem: (itemId, updates) => {
        set((state) => ({
          items: state.items.map((item) =>
            item.id === itemId
              ? { ...item, ...updates, updatedAt: nowIso() }
              : item
          ),
        }));
      },

      applyVerifiedRecoveryPatch: (patches) => {
        const validPatches = new Map(
          patches
            .filter((patch) => {
              const fields = patch.restored_fields;
              return Boolean(
                patch.item_id &&
                  patch.source_url &&
                  fields &&
                  Object.values(fields).every(
                    (value) => typeof value === "string" && value.trim() && !value.includes("\ufffd")
                  )
              );
            })
            .map((patch) => [patch.item_id, patch])
        );
        const timestamp = nowIso();
        let restored = 0;
        const touchedLibraries = new Set<string>();

        set((state) => {
          const items = state.items.map((item) => {
            const patch = validPatches.get(item.id);
            if (!patch || patch.library_id !== item.libraryId || patch.source_url !== item.sourceUrl) {
              return item;
            }
            restored += 1;
            touchedLibraries.add(item.libraryId);
            return { ...item, ...patch.restored_fields, updatedAt: timestamp };
          });

          if (!restored) return state;
          return {
            items,
            libraries: state.libraries.map((library) =>
              touchedLibraries.has(library.id) ? { ...library, updatedAt: timestamp } : library
            ),
          };
        });

        return restored;
      },

      setSelectedLibraryId: (libraryId) => set({ selectedLibraryId: libraryId }),

      saveScript: (input) => {
        const id = createId("script");
        const timestamp = nowIso();
        const script: GeneratedScript = {
          id,
          title: input.title.trim() || "未命名脚本",
          content: input.content,
          referenceLibraryId: input.referenceLibraryId,
          referenceLibraryName: input.referenceLibraryName,
          createdAt: timestamp,
          updatedAt: timestamp,
        };

        set((state) => ({
          scripts: [script, ...state.scripts],
        }));

        return id;
      },

      updateScript: (id, updates) => {
        set((state) => ({
          scripts: state.scripts.map((script) =>
            script.id === id
              ? { ...script, ...updates, updatedAt: nowIso() }
              : script
          ),
        }));
      },

      deleteScript: (id) => {
        set((state) => ({
          scripts: state.scripts.filter((script) => script.id !== id),
        }));
      },

      getLibraryItems: (libraryId) => {
        return get()
          .items.filter((item) => item.libraryId === libraryId)
          .sort((a, b) => b.createdAt.localeCompare(a.createdAt));
      },

      getSelectedLibrary: () => {
        const selectedId = get().selectedLibraryId;
        if (!selectedId) return null;
        return get().libraries.find((library) => library.id === selectedId) || null;
      },
    }),
    {
      name: "copy-library-storage",
      version: 1,
    }
  )
);
