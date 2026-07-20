import { promptHub, PromptHubError } from "$lib/server/prompthub/client";
import type { RequestHandler } from "./$types";

/**
 * REST-bridge: render a prompt template with vars (ADR-262).
 * `POST /api/prompthub/prompts/[id]/render`  body: `{ vars?: Record<string, unknown> }`
 * Returns `{ rendered: string }` — the deterministic result, never via the LLM.
 */
export const POST: RequestHandler = async ({ params, request }) => {
	let vars: Record<string, unknown> = {};
	try {
		const body = await request.json();
		if (body && typeof body === "object" && body.vars && typeof body.vars === "object") {
			vars = body.vars as Record<string, unknown>;
		}
	} catch {
		// empty / non-JSON body → render with no vars
	}

	try {
		const result = await promptHub.render(params.id, vars);
		return Response.json(result);
	} catch (err) {
		const status = err instanceof PromptHubError ? err.status : 500;
		return Response.json({ error: (err as Error).message }, { status });
	}
};
