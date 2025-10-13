// this script renders all diagrams in the 'd2' directory into the 'exports' directory
import { walk } from "https://deno.land/std/fs/walk.ts";

const inputDir = new URL("./d2", import.meta.url).pathname;
const outputDir = new URL("./exports", import.meta.url).pathname;

for await (const entry of walk(inputDir, { exts: [".d2"] })) {
    const inputFile = entry.path;
    const outputFile = `${outputDir}/${entry.name.replace(/\.d2$/, ".svg")}`;

    console.log(`Rendering: ${inputFile} -> ${outputFile}`);

    // Execute the D2 CLI command to render the file
    const process = new Deno.Command("d2", {
        args: ["-t", "200", "-l", "elk", /*"-s",*/ inputFile, outputFile],
        stdout: "piped",
        stderr: "piped",
    });

    const { code } = await process.output();
    if (code !== 0) {
        const { stderr } = await process.output();
        console.error(
            `Error rendering ${inputFile}: ${new TextDecoder().decode(stderr)}`,
        );
    } else {
        console.log(`Rendered: ${outputFile}`);
    }
}
