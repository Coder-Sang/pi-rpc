/** Real Pi RPC UI acceptance. Slash commands never submit a model request. */
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	pi.registerCommand("rpc-acceptance-ui", {
		description: "Exercise the nine RPC extension UI methods without inference",
		handler: async (args, ctx) => {
			if (args !== "manual" && args !== "cancel") throw new Error("Expected manual or cancel");
			ctx.ui.setTitle("PI_RPC_UI_TITLE");
			ctx.ui.setStatus("rpc-acceptance", "PI_RPC_UI_STATUS");
			ctx.ui.setWidget("rpc-acceptance", ["PI_RPC_UI_WIDGET", "second line"], { placement: "aboveEditor" });
			ctx.ui.setEditorText("PI_RPC_UI_EDITOR_TEXT");
			ctx.ui.notify("PI_RPC_UI_BEGIN", "info");
			const select = await ctx.ui.select("PI_RPC_UI_SELECT", ["first", "second"]);
			ctx.ui.notify(`PI_RPC_UI_ANSWER_SELECT:${JSON.stringify(select ?? null)}`, "info");
			const confirm = await ctx.ui.confirm("PI_RPC_UI_CONFIRM", "Confirm fixture only");
			ctx.ui.notify(`PI_RPC_UI_ANSWER_CONFIRM:${JSON.stringify(confirm)}`, "info");
			const input = await ctx.ui.input("PI_RPC_UI_INPUT", "fixture placeholder");
			ctx.ui.notify(`PI_RPC_UI_ANSWER_INPUT:${JSON.stringify(input ?? null)}`, "info");
			const editor = await ctx.ui.editor("PI_RPC_UI_EDITOR", "fixture prefill\nsecond line");
			ctx.ui.notify(`PI_RPC_UI_ANSWER_EDITOR:${JSON.stringify(editor ?? null)}`, "info");
			ctx.ui.notify(`PI_RPC_UI_DONE:${args}`, "info");
		},
	});
}
