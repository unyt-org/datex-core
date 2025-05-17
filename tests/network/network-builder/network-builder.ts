import cytoscape, {
    type CoseLayoutOptions,
} from "https://esm.sh/cytoscape@3.32.0";
import coseBilkent from "https://esm.sh/cytoscape-cose-bilkent@4.1.0";
import fcose from "https://esm.sh/cytoscape-fcose@2.2.0";

cytoscape.use(coseBilkent);
cytoscape.use(fcose as unknown as (cy: typeof cytoscape) => void);

type NetworkDefinition = {
    nodes: Array<{ id: string; label: string; endpoint?: string }>;
    edges: Array<{
        id: string;
        source: string;
        target: string;
        type: string;
        priority: number;
    }>;
};

type DirEntry  = {
    name: string;
    children?: DirEntry[];
} | string;

class NetworkManager {
    public static async getAllNetworks() {
        const storedNetworks = await (await fetch("/rs-lib/datex-core/tests/network/network-builder/")).json() as DirEntry[];
        return NetworkManager.resolveChildren(
            storedNetworks,
            "networks",
        )?.map((e) => e.replace(/\.json$/, ""));
    }
    public static loadNetwork(name: string) {
        return fetch(`/rs-lib/datex-core/tests/network/network-builder/networks/${name}.json`)
            .then((response) => response.json())
            .then((data: NetworkDefinition) => {
                const nodes = data.nodes?.map((node) => ({
                    group: "nodes",
                    data: node,
                })) ?? [];
                const edges = data.edges?.map((edge) => ({
                    group: "edges",
                    data: edge,
                })) ?? [];
                return { nodes, edges };
            });
    }
    private static resolveChildren(structure: DirEntry[], path: string) {
        const parts = path.split("/");
        let current = structure;
        for (const part of parts) {
            const next = current.find((item) =>
                typeof item === "object" && item.name === part
            );
            if (!next || !Array.isArray((next as {children: DirEntry[]}).children)) {
                return null;
            }
            current = (next as {children: DirEntry[]}).children;
        }
        return current as string[];
    }
}

const loadNetwork = async (name: string) => {
    try {
        const network = await NetworkManager.loadNetwork(name);
        globalThis.location.hash = name;
        cy.elements().remove();
        network.nodes.forEach((node) => {
            cy.add(node as unknown as cytoscape.ElementDefinition);
        });
        network.edges.forEach((edge) => {
            cy.add(edge as unknown as cytoscape.ElementDefinition);
        });
        layout(true);
    } catch (error) {
        console.error("Error loading network:", error);
        globalThis.location.hash = "";
    }
};
const color = (ele: cytoscape.EdgeSingular) => {
    const priority = ele.data("priority") + 1 || 0;
    return [
        "red",
        "black",
        "#303030",
        "#505050",
        "#707070",
        "#909090",
        "#A0A0A0",
        "#C0C0C0",
        "#D0D0D0",
        "#F0F0F0",
    ][priority] ||
        "#8C9BAB";
};
const container = document.getElementById("cy")!;
const cy = cytoscape({
    container,
    style: [
        {
            selector: "node",
            style: {
                "label": "data(label)",
                "text-valign": "center",
                "text-halign": "center",
                "font-size": 7,
                "shape": "roundrectangle",
                "width": 30,
                "height": 12,
                "background-color": "black",
                "border-color": "#000",
                "border-opacity": 0.5,
                "text-border-color": "#000",
                "color": "#fff",
            },
        },
        {
            selector: "node:selected",
            style: {
                "border-width": 0,
                "border-color": "transparent",
                "background-color": "#0074D9", // match default
            },
        },
        {
            selector: ":active",
            style: {
                "opacity": 0.8,
                "overlay-color": "transparent",
                "overlay-opacity": 0,
            },
        },
        {
            selector: "edge",
            style: {
                "label": "data(type)",
                "width": 1,
                "target-arrow-color": color,
                "target-arrow-shape": "triangle",
                "curve-style": "bezier",
                "line-color": color,
                "font-size": 4,
                "arrow-scale": 0.5,
                "text-background-color": "#fff",
                "text-background-opacity": 1,
                "text-background-shape": "roundrectangle",
                "text-background-padding": "2px",
            },
        },
        {
            selector: "edge:selected",
            style: {
                "color": "#0074D9",
                "target-arrow-color": "#0074D9",
                "line-color": "#0074D9",
            },
        },
    ],
    userZoomingEnabled: true,
    userPanningEnabled: true,
    boxSelectionEnabled: false,
    autoungrabify: false,
    elements: [],
});

function layout(fit = false) {
    const options: CoseLayoutOptions = {
        name: "cose",
        randomize: true,
        fit: fit,
        animate: true,
        animationDuration: 400,
        avoidOverlap: true,
        gravity: 0.25,
    };
    cy.layout({
        ...options,
        name: "fcose",
        idealEdgeLength: 80,
        tilingPaddingVertical: 100,
        tilingPaddingHorizontal: 100,
    } as unknown as cytoscape.LayoutOptions).run();
}

const getRandomName = (len = 4) => {
    const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
    const array = new Uint8Array(len);
    crypto.getRandomValues(array);
    return Array.from(array, (byte) => chars[byte % chars.length]).join("");
};

const addNode = (
    id?: string,
    position?: { x: number; y: number },
    endpoint?: string,
) => {
    if (!id) {
        id = getRandomName();
    }
    while (cy.getElementById(id).length) {
        id = getRandomName();
    }
    cy.add({
        group: "nodes",
        data: { id, label: `@${id}`, endpoint: endpoint },
        position,
    });
    if (!position) {
        layout(true);
    }
};
const remove = (node: cytoscape.SingularElementReturnValue) => {
    const id = node.data("id");
    const edges = node.connectedEdges();
    edges.forEach((edge) => {
        const target = edge.target();
        if (target.id() === id) {
            edge.remove();
        }
    });
    node.remove();
};
const removeSelected = () => {
    const collection = cy.$(":selected");
    if (collection.length > 0) {
        collection.forEach((node) => {
            remove(node);
        });
        html("edge-controls").style.display = "none";
        layout();
    }
};
document.addEventListener("keydown", (e) => {
    const collection = cy.$(":selected");
    if (
        (e.key === "Delete" || e.key === "Backspace") && collection.length > 0
    ) {
        removeSelected();
        layout();
    }
});

cy.on("tap", function (evt) {
    if (evt.target !== cy || !evt.originalEvent.shiftKey) {
        return;
    }
    addNode(undefined, evt.position);
    layout();
});
cy.on("select", "edge", (evt) => {
    openEdgeInfo(evt.target);
});

const openEdgeInfo = (edge: cytoscape.EdgeSingular) => {
    html("edge-controls").style.display = "flex";
    html("edge-id").textContent = edge.id();
    html<HTMLSelectElement>("edge-priority-select").value = edge.data(
        "priority",
    );
    html<HTMLSelectElement>("edge-type-select").value = edge.data("type") ||
        "info";
    const otherEdgeId = `${edge.data("type")}-${edge.data("target")}-${
        edge.data("source")
    }`;
    html<HTMLInputElement>("edge-bidirectional").checked =
        cy.getElementById(otherEdgeId).length > 0;

    html<HTMLInputElement>("with-known-endpoint").checked = edge.data(
        "endpoint",
    );
};
cy.on("unselect", "edge", () => {
    html("edge-controls").style.display = "none";
});
const addEdge = (
    source: string,
    target: string,
    type: string = "mockup",
    priority = 0,
) => {
    const edgeId = `${type}-${source}-${target}`;
    if (!cy.getElementById(edgeId).length) {
        return cy.add({
            group: "edges",
            data: { id: edgeId, source, target, type, priority },
        });
    }
};
const flipEdge = (edge: cytoscape.EdgeSingular) => {
    const oldSource = edge.source().id();
    const oldTarget = edge.target().id();

    const newEdge = addEdge(
        oldTarget,
        oldSource,
        edge.data("type"),
        edge.data("priority"),
    );
    if (newEdge) {
        edge.remove();
        newEdge.select();
        openEdgeInfo(newEdge);
    }
};
let sourceNode: cytoscape.NodeSingular | null = null;

cy.on("tapstart", "node", (e) => {
    if (e.originalEvent.shiftKey) {
        sourceNode = e.target;
        sourceNode?.ungrabify();
    }
});

cy.on("tapend", "node", (e) => {
    if (sourceNode && e.target !== sourceNode) {
        sourceNode.grabify();
        sourceNode.unselect();
        const targetNode = e.target;
        addEdge(
            sourceNode.id(),
            targetNode.id(),
        );
    }
    sourceNode?.grabify();
    sourceNode = null;
});
container.addEventListener(
    "contextmenu",
    (e) => e.preventDefault(),
);

document.querySelectorAll("[data-action]").forEach((el) => {
    if (!(el instanceof HTMLElement)) {
        return;
    }
    if (el instanceof HTMLSelectElement) {
        el.addEventListener("change", (e) => {
            const action = (e.target as HTMLElement).dataset.action;
            const target = e.target as HTMLSelectElement;
            const selected = target.options[target.selectedIndex];
            if (action == "edge-type-select") {
                const edge = cy.edges(":selected");
                if (edge.length === 1) {
                    edge.data("type", selected.value);
                }
            } else if (action == "edge-priority-select") {
                const edge = cy.edges(":selected");
                console.log("edge", edge, selected.value);
                if (edge.length === 1) {
                    edge.data("priority", +selected.value);
                }
            }
        });
    } else if (el instanceof HTMLInputElement) {
        el.addEventListener("input", (e) => {
            const action = (e.target as HTMLElement).dataset.action;
            const target = e.target as HTMLInputElement;
            if (action == "edge-bidirectional") {
                const edge = cy.edges(":selected");
                if (edge.length !== 1) {
                    return;
                }
                const otherEdgeId = `${edge.data("type")}-${
                    edge.data("target")
                }-${edge.data("source")}`;

                if (!target.checked) {
                    const otherEdge = cy.getElementById(otherEdgeId);
                    if (otherEdge.length) {
                        otherEdge.remove();
                    }
                } else {
                    const source = edge.data("source");
                    const target = edge.data("target");
                    addEdge(target, source, edge.data("type"));
                }
            } else if (action == "with-known-endpoint") {
                const edge = cy.edges(":selected");
                if (edge.length === 1) {
                    console.log("edge", edge, target.checked);
                    edge.data;
                    edge.data(
                        "endpoint",
                        target.checked ? "@" + edge[0].target().id() : null,
                    );
                }
            }
        });
    } else {
        el.addEventListener("click", async (e) => {
            const action = (e.target as HTMLElement).dataset.action;
            if (action === "add-node") {
                addNode();
            } else if (action === "remove") {
                removeSelected();
            } else if (action === "add-edge") {
                const selected = cy.nodes(":selected");
                if (selected.length === 2) {
                    const source = selected[0].id();
                    const target = selected[1].id();
                    addEdge(source, target);
                    selected.unselect();
                    layout();
                }
            } else if (action === "load") {
                const networks = await NetworkManager.getAllNetworks();
                const dialog = html<HTMLDialogElement>("network-dialog");
                dialog.querySelector("select")!.innerHTML = "";
                networks?.forEach((network) => {
                    const option = document.createElement("option");
                    option.value = network;
                    option.textContent = network;
                    dialog.querySelector("select")!.appendChild(option);
                });
                dialog.showModal();
            } else if (action === "load-network") {
                const dialog = html<HTMLDialogElement>("network-dialog");
                const selected = dialog.querySelector(
                    "select",
                ) as HTMLSelectElement;
                const networkName = selected.value;
                loadNetwork(networkName);
            } else if (action === "save") {
                const networkName = globalThis.location.hash.replace("#", "") ||
                    prompt("Enter network name");
                if (networkName) {
                    const nodes = cy.nodes().map((node) => node.data());
                    const edges = cy.edges().map((edge) => {
                        const data = edge.data();
                        if (!data["endpoint"]) {
                            delete data["endpoint"];
                        }
                        return data;
                    });
                    const networkData = { nodes: nodes, edges: edges };
                    // deno-lint-ignore no-explicit-any
                    const handle = await (globalThis as any).showSaveFilePicker(
                        {
                            suggestedName: `${networkName}.json`,
                            types: [{
                                description: "JSON file",
                                accept: { "application/json": [".json"] },
                            }],
                        },
                    );
                    const writable = await handle.createWritable();
                    await writable.write(
                        new Blob([JSON.stringify(networkData, null, "\t")], {
                            type: "application/json",
                        }),
                    );
                    await writable.close();
                    console.log("Saving network as:", networkName, networkData);
                }
            } else if (action === "clear") {
                cy.elements().remove();
                html("edge-controls").style.display = "none";
            } else if (action === "edge-flip") {
                const selected = cy.edges(":selected");
                if (selected.length === 1) {
                    flipEdge(selected[0]);
                }
            }
        });
    }
});

function html<T = HTMLElement>(id: string): T {
    return document.getElementById(id)! as T;
}
if (globalThis.location.hash) {
    const networkName = globalThis.location.hash.replace("#", "");
    await loadNetwork(networkName);
}
