# Vendored Proto-UI Input root

`root.proto.ts` and `types.ts` derive from Proto-UI Base Input
from base commit `5712266648fdbce2f6662636cb83f01eb849adb1` plus the
application-authored reusable `asInputEditor` and InputRoot `resetValue` patches authored for this migration. These source adaptations are tracked explicitly; compiler provenance alone does not prove that an upstream revision contains them.
The accompanying MIT license applies to both files.

The Verdant source producer admits only repository-local relative imports.
These copies keep the upstream `asInputEditor` / `asTextControl` author contract
intact rather than implementing a second Rust editor.

`../text-field.proto.ts` composes the shared editor hook with Verdant submit, cancel,
advance and selection-request policy. Native shaping, UTF-16 conversion, IME,
selection, clipboard and text undo remain in the generated Proto-UI SDK.

Verdant gives these editors direct accessible names. It does not pair an
independent Label; therefore it uses `asInputEditor`. The full upstream
`asInputRoot` still composes the real ControlLabel target protocol, whose
separate native capability is not claimed by these fields.
