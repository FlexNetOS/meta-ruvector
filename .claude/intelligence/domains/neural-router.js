import { join } from '../shared-kernel.js';

/**
 * Neural Router with enhanced intelligence
 */
class NeuralRouter {
  constructor(memory, reasoning, calibration, feedback) {
    this.memory = memory;
    this.reasoning = reasoning;
    this.calibration = calibration;
    this.feedback = feedback;
  }

  async route(task, context = {}) {
    const { fileType, crate, operation = 'edit' } = context;
    // Use underscore format to match pretrained Q-table
    const state = `${operation}_${fileType || 'file'}_in_${crate || 'project'}`;
    const agents = this.getAgentsForContext(fileType, crate);

    const suggestion = this.reasoning.getBestAction(state, agents);
    const similar = await this.memory.search(task, 3);

    let finalAgent = suggestion.action;
    let finalConf = suggestion.confidence;

    if (similar.length > 0 && similar[0].score > 0.7) {
      const pastAgent = similar[0].metadata?.agent;
      if (pastAgent && agents.includes(pastAgent)) {
        finalAgent = pastAgent;
        finalConf = Math.min(1, finalConf + 0.2);
      }
    }

    // Record for feedback tracking
    const suggestionId = `sug-${Date.now()}`;
    this.feedback.recordSuggestion(suggestionId, finalAgent, finalConf);

    return {
      recommended: finalAgent,
      confidence: finalConf,
      reason: this.buildReason(finalAgent, suggestion.reason, similar),
      alternatives: agents.filter(a => a !== finalAgent).slice(0, 3),
      context: { state, agents, similar: similar.slice(0, 2) },
      suggestionId,
      abGroup: suggestion.abGroup,
      isUncertain: suggestion.isUncertain
    };
  }

  getAgentsForContext(fileType, crate) {
    const base = ['coder', 'reviewer', 'tester'];

    const typeMap = {
      'rs': ['rust-developer', 'code-analyzer'],
      'ts': ['typescript-developer', 'backend-dev'],
      'js': ['javascript-developer', 'backend-dev'],
      'md': ['technical-writer'],
      'json': ['config-specialist'],
      'py': ['python-developer'],
      'css': ['frontend-developer'],
      'html': ['frontend-developer'],
      'tsx': ['frontend-developer'],
      'yml': ['devops-engineer'],
      'yaml': ['devops-engineer'],
      'sql': ['database-expert'],
      'sh': ['system-admin']
    };

    if (typeMap[fileType]) base.push(...typeMap[fileType]);

    // Crate-specific specializations
    if (fileType === 'rs') {
      if (crate?.includes('wasm') || crate === 'rvlite') base.push('production-validator');
      if (crate?.includes('gnn') || crate?.includes('attention') || crate === 'sona') base.push('ml-developer');
      if (crate?.includes('postgres')) base.push('backend-dev', 'system-architect');
      if (crate?.includes('mincut') || crate?.includes('graph')) base.push('system-architect');
    }

    return [...new Set(base)];
  }

  buildReason(agent, qReason, similar) {
    const parts = [];
    if (qReason === 'learned-preference') parts.push('learned from past success');
    if (similar.length > 0 && similar[0].score > 0.6) parts.push('similar past task succeeded');
    if (parts.length === 0) parts.push('default selection');
    return `${agent}: ${parts.join(', ')}`;
  }
}

export { NeuralRouter };
