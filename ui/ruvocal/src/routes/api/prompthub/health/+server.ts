import { promptHub } from "$lib/server/prompthub/client";
import type { RequestHandler } from "./$types";

/** REST-bridge liveness: `GET /api/prompthub/health` → `{ ok: boolean }` (ADR-262). */
export const GET: RequestHandler = async () => {
	const ok = await promptHub.health();
	return Response.json({ ok }, { status: ok ? 200 : 503 });
};
