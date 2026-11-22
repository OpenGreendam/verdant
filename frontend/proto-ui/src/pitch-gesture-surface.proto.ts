import { definePrototype, tw } from '@proto.ui/core';

export default definePrototype({
  name: 'verdant-pitch-gesture-surface',
  setup(def) {
    // The native slot owns score drawing and music-coordinate translation.
    // Note and device planners are separate compilation units sharing this SDK.
    def.feedback.style.use(tw('w-full h-full overflow-hidden'));
    return (renderer) => renderer.slot();
  },
});
