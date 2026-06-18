/**
 * prompt_hub REST-bridge client (ADR-262).
 *
 * Server-to-server HTTP client for `prompthub-server` (axum REST API).
 * This is the deliberate alternative to exposing prompt_hub over MCP: prompt
 * rendering / bundle generation are deterministic data operations, so routing
 * them through the LLM (MCP) is pure token waste. The bridge keeps the model
 * out of the loop — it only ever sees a *rendered prompt*, never tool chatter.
 *
 * Reads `PROMPTHUB_URL` (default http://127.0.0.1:8077) from config. Used only
 * from `+server.ts` endpoints (never the browser).
 */
import { config } from "$lib/server/config";

/** prompthub-server's uniform response envelope: `{ success, data?, error? }`. */
interface ApiEnvelope<T> {
	success: boolean;
	data?: T;
	error?: { code?: string; message?: string } | string | null;
}

export interface PromptSummary {
	id: string;
	name: string;
	version?: string;
	status?: string;
	domain?: string;
	tags?: string[];
	system_prompt?: string;
	user_template?: string;
	created_at?: string;
	updated_at?: string;
}

export interface PromptList {
	items: PromptSummary[];
	total: number;
	page: number;
	per_page: number;
}

export interface RenderResult {
	rendered: string;
}

/** SwarmBundle as emitted by `GET /api/v1/swarm/bundle`. */
export interface SwarmBundle {
	workflow_id: string;
	prompt_count: number;
	roles: Record<string, Array<{ id: string; name: string; system_prompt?: string }>>;
	consistency_report: unknown[];
	evolution_suggestions: unknown[];
}

export class PromptHubError extends Error {
	constructor(
		message: string,
		readonly status: number
	) {
		super(message);
		this.name = "PromptHubError";
	}
}

function baseUrl(): string {
	return (config.PROMPTHUB_URL || "http://127.0.0.1:8077").replace(/\/$/, "");
}

function timeoutMs(): number {
	const raw = Number(config.MCP_TOOL_TIMEOUT_MS);
	return Number.isFinite(raw) && raw > 0 ? raw : 15_000;
}

/** Perform a request and unwrap the `{ success, data }` envelope. */
async function request<T>(
	path: string,
	init?: RequestInit & { query?: Record<string, string | number | undefined> }
): Promise<T> {
	const url = new URL(baseUrl() + path);
	if (init?.query) {
		for (const [k, v] of Object.entries(init.query)) {
			if (v !== undefined && v !== "") url.searchParams.set(k, String(v));
		}
	}

	const ctrl = new AbortController();
	const timer = setTimeout(() => ctrl.abort(), timeoutMs());
	let res: Response;
	try {
		res = await fetch(url, {
			...init,
			signal: ctrl.signal,
			headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
		});
	} catch (err) {
		throw new PromptHubError(
			`prompt_hub unreachable at ${baseUrl()} (${(err as Error).message})`,
			502
		);
	} finally {
		clearTimeout(timer);
	}

	let body: ApiEnvelope<T> | undefined;
	try {
		body = (await res.json()) as ApiEnvelope<T>;
	} catch {
		// non-JSON body
	}

	if (!res.ok || !body?.success) {
		const msg =
			(typeof body?.error === "string" ? body.error : body?.error?.message) ??
			`prompt_hub returned ${res.status}`;
		throw new PromptHubError(msg, res.status || 502);
	}
	return body.data as T;
}

export const promptHub = {
	/** `GET /api/v1/prompts` — paginated list. */
	list(opts: { page?: number; perPage?: number; domain?: string } = {}): Promise<PromptList> {
		return request<PromptList>("/api/v1/prompts", {
			query: { page: opts.page, per_page: opts.perPage, domain: opts.domain },
		});
	},

	/** `GET /api/v1/prompts/search` — full-text / semantic search. */
	search(q: string, opts: { mode?: string; page?: number; perPage?: number } = {}): Promise<PromptList> {
		return request<PromptList>("/api/v1/prompts/search", {
			query: { q, mode: opts.mode, page: opts.page, per_page: opts.perPage },
		});
	},

	/** `GET /api/v1/prompts/{id}` — single prompt. */
	get(id: string): Promise<PromptSummary> {
		return request<PromptSummary>(`/api/v1/prompts/${encodeURIComponent(id)}`);
	},

	/** `POST /api/v1/prompts/{id}/render` — render a template with vars. */
	render(id: string, vars: Record<string, unknown> = {}): Promise<RenderResult> {
		return request<RenderResult>(`/api/v1/prompts/${encodeURIComponent(id)}/render`, {
			method: "POST",
			body: JSON.stringify({ vars }),
		});
	},

	/** `GET /api/v1/swarm/bundle` — SwarmBundle for the handoff.task.v1 flow. */
	bundle(): Promise<SwarmBundle> {
		return request<SwarmBundle>("/api/v1/swarm/bundle");
	},

	/** `GET /health` — liveness probe (returns true/false, never throws). */
	async health(): Promise<boolean> {
		try {
			const res = await fetch(baseUrl() + "/health", {
				signal: AbortSignal.timeout(3_000),
			});
			return res.ok;
		} catch {
			return false;
		}
	},
};
