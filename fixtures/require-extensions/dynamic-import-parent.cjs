module.exports = import("./dynamic-import-child.cjs").then((mod) => mod.default ?? mod);
