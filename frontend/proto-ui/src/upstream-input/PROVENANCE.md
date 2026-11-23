# Vendored Proto-UI Input editor

`root.proto.ts` and `types.ts` are exact copies of
`packages/prototypes/base/src/input/` at Proto-UI commit
[`3b0d2dab0555ef6a7c4869c2d2b45ec531b77861`](https://github.com/Proto-UI/Proto-UI/tree/3b0d2dab0555ef6a7c4869c2d2b45ec531b77861/packages/prototypes/base/src/input).
The accompanying MIT license is copied from that same commit.
The compiler pin binds this identical immutable upstream revision.

Verdant's source producer admits repository-local relative imports. These
copies retain the upstream shared `asInputEditor` / `asTextControl` author
contract, including reset and controlled-value behavior. Native shaping,
UTF-16 conversion, IME, selection, clipboard and text undo remain in the
generated Proto-UI SDK.

`../text-field.proto.ts` composes the editor with Verdant submit, cancel,
advance and selection-request policy. These fields provide direct accessible
names. The complete upstream `asInputRoot` still composes the real ControlLabel
target protocol; the separate native Label association is not claimed here.
