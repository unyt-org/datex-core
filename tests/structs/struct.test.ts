import dxb from "example-dxb" with {
    type: "json",
};

import { parseAndPackStructure, type StructureDefinition } from "@unyt/speck";
import { diff } from "@opentf/obj-diff";

function resolvePath(
    object: Record<string, unknown>,
    path: (string | number)[],
): unknown {
    return path.reduce(
        (obj: any, key) => (obj && key in obj ? obj[key] : undefined),
        object,
    );
}

const BASE = new URL("./", import.meta.url);
const IGNORED_PATHS = [
    "body",
];

for await (const dirEntry of Deno.readDir(BASE)) {
    if (!dirEntry.isDirectory) continue;

    Deno.test(`struct: ${dirEntry.name}`, async () => {
        const binData = await Deno.readFile(
            new URL(`${dirEntry.name}/block.bin`, BASE),
        );
        const jsonData = JSON.parse(
            await Deno.readTextFile(
                new URL(`${dirEntry.name}/block.json`, BASE),
            ),
        );

        const parsed = parseAndPackStructure(
            dxb as StructureDefinition,
            binData,
        );
        const differences = diff(jsonData, parsed);

        const errors: string[] = [];

        for (const difference of differences) {
            const path = difference.p.join(".");
            const type =
                (["Missing field", "Additional", "Different value"] as const)[
                    difference.t
                ];
            if (IGNORED_PATHS.includes(path) || difference.v === null) continue;

            if (path.startsWith("routing_header.receivers_with_keys")) {
                const [expectedEndpoint, expectedKey] =
                    resolvePath(jsonData, difference.p) as [string, number[]] ??
                        [];
                const { receiver: parsedEndpoint, key: parsedKey } = difference
                    .v as any;
                if (expectedEndpoint !== parsedEndpoint) {
                    throw new Error(
                        `Difference at '${path}': Expected '${expectedEndpoint}', found '${parsedEndpoint}'`,
                    );
                }
                const hexKey = expectedKey.map((ba: number) =>
                    ba.toString(16).padStart(2, "0")
                )
                    .join("");
                if (parsedKey !== hexKey) {
                    errors.push(
                        `Difference at '${path}': Expected key '${hexKey}', found '${parsedKey}'`,
                    );
                }
                continue;
            }

            errors.push(
                `${type} at '${path}': Expected '${
                    resolvePath(jsonData, difference.p) ?? "-"
                }', found '${difference.v}'`,
            );
        }

        if (errors.length > 0) {
            throw new Error(errors.join("\n"));
        }
    });
}
