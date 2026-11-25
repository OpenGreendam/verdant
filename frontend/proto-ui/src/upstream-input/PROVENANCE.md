# Vendored Proto-UI Input editor

`root.proto.ts` and `types.ts` are exact copies of
`packages/prototypes/base/src/input/` at Proto-UI commit
[`4e499ae25e825e130e740f16af66d70b23a743cf`](https://github.com/Proto-UI/Proto-UI/tree/4e499ae25e825e130e740f16af66d70b23a743cf/packages/prototypes/base/src/input).
The accompanying MIT license is copied from that same commit.
These vendored files have their own immutable source revision in
`provenance.json`. The compiler is independently pinned by
`../../compiler.pin.json`; upgrading that compiler does not replace these
vendored Input files.

Verdant's source producer admits repository-local relative imports. These
copies retain the upstream shared `asInputEditor` / `asTextControl` author
contract, including reset and controlled-value behavior. Native shaping,
UTF-16 conversion, IME, selection, clipboard and text undo remain in the
generated Proto-UI SDK.

`../text-field.proto.ts` composes the editor with Verdant submit, cancel,
advance and selection-request policy. These fields provide direct accessible
names. The complete upstream `asInputRoot` still composes the real ControlLabel
target protocol; the separate native Label association is not claimed here.
