// Explicitly loaded only by the opt-in live acceptance example.
import { Type } from "@earendil-works/pi-ai";
import { existsSync } from "node:fs";
import { join } from "node:path";

export default function (pi: any) {
  pi.registerTool({
    name: "rpc_acceptance_wait",
    label: "RPC acceptance wait",
    description: "Wait for the local RPC acceptance harness to release this tool, then return PI_TOOL_OK. Call this only once per user request.",
    parameters: Type.Object({}),
    async execute(_id: string, _params: unknown, signal: AbortSignal, onUpdate: any, ctx: any) {
      onUpdate({ content: [{ type: "text", text: "PI_TOOL_WAITING" }], details: {} });
      const deadline = Date.now() + 30000;
      while (!existsSync(join(ctx.cwd, "rpc-tool-release"))) {
        if (signal.aborted) throw new Error("RPC acceptance tool aborted");
        if (Date.now() >= deadline) throw new Error("RPC acceptance tool deadline");
        await new Promise((resolve) => setTimeout(resolve, 20));
      }
      return { content: [{ type: "text", text: "PI_TOOL_OK" }], details: { released: true } };
    },
  });
}
