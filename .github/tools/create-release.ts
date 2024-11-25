const [type] = Deno.args;

const ghOutput = Deno.env.get("GITHUB_OUTPUT")!;
if (!ghOutput) {
    throw new Error("Can not find GITHUB_OUTPUT environment variable");
}

if (!["major", "minor", "patch"].includes(type)) {
    throw new Error(
        "Invalid version bump type. Use 'major', 'minor', or 'patch'.",
    );
}

const cargoTomlPath = "./Cargo.toml";
const cargoToml = await Deno.readTextFile(cargoTomlPath);

// Extract version
const versionRegex = /version\s*=\s*"(\d+)\.(\d+)\.(\d+)"/;
const match = versionRegex.exec(cargoToml);
if (!match) {
    throw new Error("Version not found in Cargo.toml");
}

let [major, minor, patch] = match.slice(1).map(Number);

switch (type) {
    case "major":
        major++;
        minor = 0;
        patch = 0;
        break;
    case "minor":
        minor++;
        patch = 0;
        break;
    case "patch":
        patch++;
        break;
}

const newVersion = `${major}.${minor}.${patch}`;
const updatedCargoToml = cargoToml.replace(
    versionRegex,
    `version = "${newVersion}"`,
);
await Deno.writeTextFile(cargoTomlPath, updatedCargoToml);
await Deno.writeTextFile(ghOutput, `NEW_VERSION=${newVersion}`);

console.log(`Version updated to ${newVersion}`, ghOutput);