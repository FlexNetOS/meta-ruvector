import { readFileSync, writeFileSync, existsSync, FACTS_FILE } from '../shared-kernel.js';

/**
 * MemoryPatterns — the AgentDB "memory patterns" bounded context.
 *
 * Layers the four patterns from the agentdb-memory-patterns skill on top of the
 * native-HNSW VectorMemory (which it composes, never replaces):
 *
 *   1. Session memory   — sessionId-scoped conversation history (embedded, so it
 *                         is also semantically searchable through VectorMemory).
 *   2. Long-term facts  — structured category/key/value records with confidence,
 *                         persisted separately in facts.json (O(1) keyed lookup,
 *                         not embedded — facts are exact, not fuzzy).
 *   3. Consolidation    — importance/recency pruning of the semantic store.
 *   4. Hierarchical org — immediate / shortTerm / longTerm / semantic views.
 *
 * Session messages live in VectorMemory's store (type "session"); facts live in
 * their own file. That split is deliberate: semantic recall over conversation is
 * the point of embedding messages, while facts must be retrievable by exact key.
 */
class MemoryPatterns {
  /**
   * @param {import('./vector-memory.js').VectorMemory} vectorMemory the shared
   *   semantic store. Session messages are stored through it, so they share the
   *   one native-HNSW index and the one memory.json file.
   */
  constructor(vectorMemory) {
    this.memory = vectorMemory;
    this.facts = this.loadFacts();
  }

  // --- persistence ---------------------------------------------------------

  loadFacts() {
    if (existsSync(FACTS_FILE)) {
      try { return JSON.parse(readFileSync(FACTS_FILE, 'utf-8')); }
      catch { return {}; }
    }
    return {};
  }

  saveFacts() {
    writeFileSync(FACTS_FILE, JSON.stringify(this.facts, null, 2));
  }

  // --- 1. Session memory ---------------------------------------------------

  /**
   * Store one conversation turn. Embedded via VectorMemory, so it counts toward
   * semantic search; the sessionId/role/seq live in metadata for replay.
   * @returns {Promise<string>} the memory id.
   */
  async storeMessage(sessionId, role, content, metadata = {}) {
    if (!sessionId) throw new Error('storeMessage requires a sessionId');
    const seq = this.getSessionHistory(sessionId, Infinity).length;
    return this.memory.store('session', content, { ...metadata, sessionId, role, seq });
  }

  /**
   * Chronological history for a session (oldest → newest), capped at `limit`
   * most-recent turns. Reads from the shared store; no separate index needed.
   */
  getSessionHistory(sessionId, limit = 20) {
    const msgs = this.memory.memories
      .filter(m => m.type === 'session' && m.metadata?.sessionId === sessionId)
      .sort((a, b) => (a.metadata?.seq ?? 0) - (b.metadata?.seq ?? 0));
    return limit === Infinity ? msgs : msgs.slice(-limit);
  }

  /** Distinct sessions with turn counts and last-activity timestamp. */
  listSessions() {
    const sessions = {};
    for (const m of this.memory.memories) {
      if (m.type !== 'session') continue;
      const sid = m.metadata?.sessionId;
      if (!sid) continue;
      const s = sessions[sid] || (sessions[sid] = { sessionId: sid, turns: 0, lastActivity: null });
      s.turns++;
      const ts = m.metadata?.timestamp;
      if (ts && (!s.lastActivity || ts > s.lastActivity)) s.lastActivity = ts;
    }
    return Object.values(sessions);
  }

  // --- 2. Long-term facts --------------------------------------------------

  /**
   * Upsert a structured fact. Re-storing the same category/key updates the value
   * and confidence and bumps a reinforcement counter (how often it was asserted).
   */
  storeFact(category, key, value, { confidence = 1.0, source = 'explicit' } = {}) {
    if (!category || !key) throw new Error('storeFact requires category and key');
    const id = `${category}:${key}`;
    const existing = this.facts[id];
    const now = new Date().toISOString();
    this.facts[id] = {
      category, key, value, confidence, source,
      reinforced: (existing?.reinforced ?? 0) + 1,
      createdAt: existing?.createdAt ?? now,
      updatedAt: now,
    };
    this.saveFacts();
    return id;
  }

  /** All facts in a category (or every fact when category is omitted). */
  getFacts(category = null) {
    const all = Object.values(this.facts);
    return category ? all.filter(f => f.category === category) : all;
  }

  /** A single fact by exact category/key, or null. */
  getFact(category, key) {
    return this.facts[`${category}:${key}`] ?? null;
  }

  /** Remove a fact. Returns true if one was deleted. */
  forgetFact(category, key) {
    const id = `${category}:${key}`;
    if (!(id in this.facts)) return false;
    delete this.facts[id];
    this.saveFacts();
    return true;
  }

  // --- 3. Memory consolidation ---------------------------------------------

  /**
   * Importance score for a single semantic memory: a recency-decayed blend of
   * confidence and reinforcement. Higher = keep.
   */
  scoreMemory(mem, now, halfLifeMs) {
    const ts = mem.metadata?.timestamp ? Date.parse(mem.metadata.timestamp) : now;
    const ageMs = Math.max(0, now - ts);
    const recency = Math.pow(0.5, ageMs / halfLifeMs); // 1 → 0 over many half-lives
    const confidence = mem.metadata?.confidence ?? 0.5;
    const reinforced = Math.min(1, (mem.metadata?.reinforced ?? 0) / 5);
    return 0.5 * recency + 0.3 * confidence + 0.2 * reinforced;
  }

  /**
   * Prune the semantic store. Drops memories scoring below `minScore`, then caps
   * the survivors at `maxSize` (highest score wins). `strategy: 'recency'` scores
   * purely by age; 'importance' (default) uses the blended score above.
   *
   * Note: pruned ids may remain in the native HNSW index until the next rebuild,
   * but search maps candidates back through `memories` and so silently drops them.
   * @returns {{before:number, after:number, removed:number}}
   */
  consolidate({ strategy = 'importance', maxSize = 10000, minScore = 0.0, halfLifeMs = 7 * 24 * 60 * 60 * 1000 } = {}) {
    const before = this.memory.memories.length;
    const now = Date.now();
    const scoreOf = strategy === 'recency'
      ? (m) => this.scoreMemory({ ...m, metadata: { ...m.metadata, confidence: 0, reinforced: 0 } }, now, halfLifeMs)
      : (m) => this.scoreMemory(m, now, halfLifeMs);

    let survivors = this.memory.memories
      .map(m => ({ m, score: scoreOf(m) }))
      .filter(x => x.score >= minScore)
      .sort((a, b) => b.score - a.score);

    if (survivors.length > maxSize) survivors = survivors.slice(0, maxSize);

    // Preserve original insertion order among survivors for stable history.
    const keep = new Set(survivors.map(x => x.m.id));
    this.memory.memories = this.memory.memories.filter(m => keep.has(m.id));
    this.memory.saveMemories();

    const after = this.memory.memories.length;
    return { before, after, removed: before - after };
  }

  // --- 4. Hierarchical organization ---------------------------------------

  /**
   * Assemble a layered context view for a query:
   *   - immediate: the last `immediateN` turns of `sessionId` (if given)
   *   - shortTerm: full history of `sessionId`
   *   - longTerm:  facts (optionally filtered by `factCategory`)
   *   - semantic:  top-`semanticK` HNSW matches for `query`
   */
  async organize(query, { sessionId = null, immediateN = 10, factCategory = null, semanticK = 5 } = {}) {
    const shortTerm = sessionId ? this.getSessionHistory(sessionId, Infinity) : [];
    return {
      immediate: sessionId ? this.getSessionHistory(sessionId, immediateN) : [],
      shortTerm,
      longTerm: this.getFacts(factCategory),
      semantic: query ? await this.memory.search(query, semanticK) : [],
    };
  }

  // --- stats ---------------------------------------------------------------

  getStats() {
    return {
      sessions: this.listSessions().length,
      sessionMessages: this.memory.memories.filter(m => m.type === 'session').length,
      facts: Object.keys(this.facts).length,
      factCategories: [...new Set(this.getFacts().map(f => f.category))],
    };
  }
}

export { MemoryPatterns };
