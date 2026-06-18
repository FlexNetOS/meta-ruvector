import { promptHub, PromptHubError } from "$lib/server/prompthub/client";
import type { RequestHandler } from "./$types";

/**
 * REST-bridge: generate a SwarmBundle for the handoff.task.v1 flow (ADR-262).
 * `GET /api/prompthub/swarm/bundle`
 */
export const GET: RequestHandler = async () => {
	try {
		const bundle = await promptHub.bundle();
		return Response.json(bundle);
	} catch (err) {
		const status = err instanceof PromptHubError ? err.status : 500;
		return Response.json({ error: (err as Error).message }, { status });
	}
};
