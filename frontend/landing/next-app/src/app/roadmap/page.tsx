import ScrollReveal from '../components/ScrollReveal';

export default function Roadmap() {
  return (
    <div className="space-y-12">
      <ScrollReveal>
        <h1 className="text-4xl font-bold mb-8">Evolution Roadmap</h1>
        <p className="text-lg mb-12">
          Our journey towards a synergistic future of human-AI creativity unfolds across 
          multiple phases, each building upon the last to create an ecosystem of 
          unprecedented creative potential.
        </p>
      </ScrollReveal>

      <div className="space-y-16">
        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Phase 1: Foundation</h2>
          <p className="mb-4">Establishing the core infrastructure for AI-human collaboration:</p>
          <ul className="list-disc pl-6 space-y-2">
            <li>Neural network integration framework</li>
            <li>Creative workflow optimization</li>
            <li>Adaptive learning systems</li>
            <li>User experience refinement</li>
          </ul>
        </ScrollReveal>

        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Phase 2: Expansion</h2>
          <p className="mb-4">Broadening the creative possibilities:</p>
          <ul className="list-disc pl-6 space-y-2">
            <li>Advanced pattern recognition</li>
            <li>Multi-modal content generation</li>
            <li>Collaborative filtering systems</li>
            <li>Real-time feedback loops</li>
          </ul>
        </ScrollReveal>

        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Phase 3: Synthesis</h2>
          <p className="mb-4">Achieving seamless integration:</p>
          <ul className="list-disc pl-6 space-y-2">
            <li>Quantum computing integration</li>
            <li>Emotional intelligence algorithms</li>
            <li>Creative consciousness mapping</li>
            <li>Universal style transfer</li>
          </ul>
        </ScrollReveal>
      </div>
    </div>
  );
} 