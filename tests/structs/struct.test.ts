import dxb from "https://raw.githubusercontent.com/unyt-org/speck/refs/heads/main/examples/dxb.json" with {
  type: "json",
};

import {
  parseAndPackStructure,
  type StructureDefinition,
} from "jsr:@unyt/speck";
import { diff } from "jsr:@opentf/obj-diff";

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
  "routing_header.magic_number",
  "body",
];

for await (const dirEntry of Deno.readDir(BASE)) {
  if (!dirEntry.isDirectory) continue;

  Deno.test(`struct: ${dirEntry.name}`, async () => {
    const binData = await Deno.readFile(
      new URL(`${dirEntry.name}/block.bin`, BASE),
    );
    const jsonData = JSON.parse(
      await Deno.readTextFile(new URL(`${dirEntry.name}/block.json`, BASE)),
    );

    const parsed = parseAndPackStructure(dxb as StructureDefinition, binData);
    const differences = diff(jsonData, parsed);

    const errors: string[] = [];

    for (const difference of differences) {
      const path = difference.p.join(".");
      const type =
        (["Missing field", "Additional", "Different value"] as const)[
          difference.t
        ];

      if (IGNORED_PATHS.includes(path) || type === "Additional") continue;

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
