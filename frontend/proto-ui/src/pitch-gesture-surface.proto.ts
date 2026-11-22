import { definePrototype, tw } from '@proto.ui/core';

export default definePrototype({
  name: 'verdant-pitch-gesture-surface',
  setup(def) {
    // The native slot owns score drawing and music-coordinate translation.
    // Pitch and normalized-knob share this SDK; data planners do not depend on it.
    def.feedback.style.use(tw('w-full h-full overflow-hidden'));
    return (renderer) => renderer.slot();
  },
});
