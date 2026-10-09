/** Synthetic upstream fault injection into REAL Pi's retry loop. No HTTP or paid calls.
 * This is NOT evidence of a real Anthropic outage or its HTTP adapter behavior.
 */
import { createAssistantMessageEventStream, type AssistantMessage } from "@earendil-works/pi-ai";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	let calls = 0;
	let scenario = "recover";
	pi.registerCommand("rpc-acceptance-retry", {
		description: "Arm the in-memory synthetic retry fixture",
		handler: async (args, ctx) => {
			if (args !== "recover" && args !== "abort" && args !== "disabled" && args !== "compact") throw new Error("Unknown retry scenario");
			scenario = args;
			calls = 0;
			ctx.ui.notify(`PI_RPC_RETRY_ARMED:${scenario}`, "info");
		},
	});
	pi.on("session_before_compact", async (event, ctx) => {
		// Never fall back to a paid summarizer, even if an unexpected boundary occurs.
		if (scenario !== "compact") return { cancel: true };
		ctx.ui.notify("PI_RPC_SYNTHETIC_COMPACTION_HOOK", "info");
		return { compaction: {
			summary: "PI_RPC_EXTENSION_PROVIDED_SUMMARY",
			firstKeptEntryId: event.preparation.firstKeptEntryId,
			tokensBefore: event.preparation.tokensBefore,
			details: { synthetic: true, summarySource: "extension-fixture" },
		} };
	});
	pi.registerProvider("rpc-acceptance-synthetic", {
		baseUrl: "http://127.0.0.1:1/never-used",
		apiKey: "synthetic-fixture-not-a-credential",
		api: "rpc-acceptance-synthetic-api",
		models: [{
			id: "retry-fixture", name: "In-memory synthetic 429/503 retry fixture",
			reasoning: false, input: ["text"],
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
			contextWindow: 200000, maxTokens: 1024,
		}, {
			id: "compact-fixture", name: "Synthetic usage threshold compaction fixture",
			reasoning: false, input: ["text"],
			cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
			contextWindow: 50000, maxTokens: 24000,
		}],
		streamSimple(model, _context, options) {
			const stream = createAssistantMessageEventStream();
			const call = ++calls;
			const output: AssistantMessage = {
				role: "assistant", content: [], api: model.api, provider: model.provider, model: model.id,
				usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,
					cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
				stopReason: "pending", timestamp: Date.now(),
			};
			void (async () => {
				try {
					const status = scenario === "compact" || (scenario === "recover" && call > 2) ? 200 : call === 1 ? 429 : 503;
					await options?.onPayload?.({ synthetic: true, scenario, call }, model);
					await options?.onResponse?.({ status, headers: {} }, model);
					stream.push({ type: "start", partial: output });
					if (status !== 200) {
						output.stopReason = "error";
						output.errorMessage = `${status} ${status === 429 ? "rate limit" : "service unavailable"} PI_RPC_SYNTHETIC_FAULT:${scenario}:${call}`;
						stream.push({ type: "error", reason: "error", error: output });
					} else {
						output.content = [{ type: "text", text: scenario === "compact" ? "PI_RPC_SYNTHETIC_LARGE_OUTPUT:" + "x".repeat(88000) : `PI_RPC_SYNTHETIC_RECOVERED:${call}` }];
						if (scenario === "compact") {
							// Synthetic usage crosses the default threshold without editing settings.
							output.usage.input = 13000;
							output.usage.output = 22000;
							output.usage.totalTokens = 35000;
						}
						output.stopReason = "stop";
						stream.push({ type: "done", reason: "stop", message: output });
					}
				} catch {
					output.stopReason = "error";
					output.errorMessage = "Synthetic fixture callback failure";
					stream.push({ type: "error", reason: "error", error: output });
				} finally { stream.end(); }
			})();
			return stream;
		},
	});
}
