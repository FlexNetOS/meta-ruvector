import { promptHub, PromptHubError } from "$lib/server/prompthub/client";
import type { RequestHandler } from "./$types";

/** REST-bridge: fetch a single prompt. `GET /api/prompthub/prompts/[id]` (ADR-262). */
export const GET: RequestHandler = async ({ params }) => {
	try {
		const prompt = await promptHub.get(params.id);
		return Response.json(prompt);
	} catch (err) {
		const status = err instanceof PromptHubError ? err.status : 500;
		return Response.json({ error: (err as Error).message }, { status });
	}
};
