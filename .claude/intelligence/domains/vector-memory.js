import { readFileSync, writeFileSync, existsSync, join, MEMORY_FILE, HNSW_DIR, poincareDistance, textToEmbedding, cosineSimilarity, VectorDB, ruvectorAvailable } from '../shared-kernel.js';

/**
 * Vector Memory with Native HNSW + Hyperbolic distance option
 */
class VectorMemory {
  constructor(options = {}) {
    this.useHyperbolic = options.hyperbolic ?? true;
    this.curvature = options.curvature ?? 1.0;
    this.db = null;
    this.memories = this.loadMemories();
    this.dimensions = 128;
  }

  loadMemories() {
    if (existsSync(MEMORY_FILE)) {
      try { return JSON.parse(readFileSync(MEMORY_FILE, 'utf-8')); }
      catch { return []; }
    }
    return [];
  }

  saveMemories() {
    writeFileSync(MEMORY_FILE, JSON.stringify(this.memories, null, 2));
  }

  async init() {
    if (ruvectorAvailable && VectorDB && !this.db) {
      try {
        this.db = new VectorDB({
          dimensions: this.dimensions,
          distanceMetric: 'Cosine', // Native HNSW uses cosine, we post-process with hyperbolic
          // Explicit, isolated, DIMENSION-KEYED store. Without storagePath the crate
          // defaults to "./ruvector.db" in the CWD, where a stale store (e.g. a 384-dim
          // one) silently overrides `dimensions` and breaks every insert. Keying the
          // filename by dimension guarantees a change can never collide with an old store.
          storagePath: join(HNSW_DIR, `ruvector-hnsw-${this.dimensions}.db`),
          hnswConfig: { m: 16, efConstruction: 200, efSearch: 100, maxElements: 50000 }
        });

        // Rebuild index from stored memories
        let rebuilt = 0;
        for (const mem of this.memories) {
          if (mem.embedding) {
            await this.db.insert({ id: mem.id, vector: new Float32Array(mem.embedding) });
            rebuilt++;
          }
        }
        console.error(`📊 VectorDB rebuilt with ${rebuilt} memories (HNSW index ready)`);
      } catch (e) {
        console.error('VectorDB init failed:', e.message);
        this.db = null;
      }
    }
  }

  async store(type, content, metadata = {}) {
    const id = `${type}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const embedding = textToEmbedding(content, this.dimensions);

    const memory = {
      id, type, content, embedding,
      metadata: { ...metadata, timestamp: new Date().toISOString() }
    };

    this.memories.push(memory);
    if (this.db) {
      try { await this.db.insert({ id, vector: new Float32Array(embedding) }); }
      catch (e) { /* fallback works */ }
    }

    this.saveMemories();
    return id;
  }

  async search(query, limit = 5) {
    const queryEmbedding = textToEmbedding(query, this.dimensions);

    // Use native HNSW for candidate retrieval
    let candidates = this.memories;
    if (this.db) {
      try {
        const results = await this.db.search({
          vector: new Float32Array(queryEmbedding),
          k: Math.min(limit * 3, 50) // Get more candidates for reranking
        });
        candidates = results.map(r => this.memories.find(m => m.id === r.id)).filter(Boolean);
      } catch (e) { /* use all memories */ }
    }

    // Rerank with hyperbolic distance if enabled
    const scored = candidates.map(mem => {
      let score;
      if (this.useHyperbolic && mem.embedding) {
        const dist = poincareDistance(queryEmbedding, mem.embedding, this.curvature);
        score = 1 / (1 + dist); // Convert distance to similarity
      } else {
        score = cosineSimilarity(queryEmbedding, mem.embedding || []);
      }
      return { ...mem, score };
    });

    return scored.sort((a, b) => b.score - a.score).slice(0, limit);
  }

  getStats() {
    const typeCount = {};
    for (const mem of this.memories) {
      typeCount[mem.type] = (typeCount[mem.type] || 0) + 1;
    }
    return {
      total: this.memories.length,
      byType: typeCount,
      usingNativeHNSW: !!this.db,
      usingHyperbolic: this.useHyperbolic
    };
  }
}

export { VectorMemory };
