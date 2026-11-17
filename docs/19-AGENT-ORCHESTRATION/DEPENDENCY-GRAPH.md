# DEPENDENCY_GRAPH (extract)
D16/D17 (identity/tenant) -> all. D02 -> D01. D03 -> D02. D04 -> D03. D05 -> D04.
D06 -> D04+D05. D08/D09 -> D03. D10 -> D09 (guardrailed). D13 -> D06. D14/D15 -> D03.
D07 -> D04+D10(AI never certifies). D21 -> D16. D22/D11 cross-cutting. D23 -> D04.
Critical path: D16/D17 -> D02 -> D03 -> D04 -> D05 -> D06 -> D09/D10.
