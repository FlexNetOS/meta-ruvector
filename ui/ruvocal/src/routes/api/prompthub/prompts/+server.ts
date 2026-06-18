import { promptHub, PromptHubError } from "$lib/server/prompthub/client";
import type { RequestHandler } from "./$types";

/**
 * REST-bridge: list or search prompts from prompthub-server (ADR-262).
 * `GET /api/prompthub/prompts?q=&domain=&page=&per_page=`
 * If `q` is present, performs a search; otherwise a paginated list.
 */
export const GET: RequestHandler = async ({ url }) => {
	const q = url.searchParams.get("q")?.trim();
	const page = Number(url.searchParams.get("page")) || undefined;
	const perPage = Number(url.searchParams.get("per_page")) || undefined;
	const domain = url.searchParams.get("domain") ?? undefined;
	const mode = url.searchParams.get("mode") ?? undefined;

	try {
		const result = q
			? await promptHub.search(q, { mode, page, perPage })
			: await promptHub.list({ page, perPage, domain });
		return Response.json(result);
	} catch (err) {
		const status = err instanceof PromptHubError ? err.status : 500;
		return Response.json({ error: (err as Error).message }, { status });
	}
};
