# Vendored Proto-UI Base Input

`root.proto.ts` and `types.ts` are byte-for-byte copies of
`packages/prototypes/base/src/input/` at Proto-UI commit
`26b9258dbb1ce580ec05763e78b7b4fc5f89e709`. The accompanying MIT license
applies to both files. `provenance.json` records their upstream paths and
SHA-256 identities; the source producer also records them in its source graph.

The Verdant producer admits repository-local relative imports. These copies
preserve the complete official `asInputRoot` contract, including its real
ControlLabel target, `asTextControl` editor, and lifecycle-safe `resetValue`.
The reset capability was contributed upstream as part of this migration;
there is no private reduced editor hook in this copy.

`../text-field.proto.ts` composes `asInputRoot` with Verdant submit, cancel,
advance, numeric-step and selection-request policy. Native shaping, UTF-16
conversion, IME, selection, clipboard and text undo remain in the generated
Proto-UI SDK. The fields provide direct accessible names while retaining the
upstream label-association contract.
