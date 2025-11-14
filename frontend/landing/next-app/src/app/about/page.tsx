import ScrollReveal from '../components/ScrollReveal';

export default function About() {
  return (
    <div className="space-y-12">
      <ScrollReveal>
        <h1 className="text-4xl font-bold mb-8">Pioneering Digital Creation</h1>
        <p className="text-lg mb-6">
          At author.works, we&apos;re architecting a future where human creativity 
          and artificial intelligence dance in perfect harmony. Our mission is to unlock 
          the boundless potential that exists at the intersection of human intuition 
          and computational intelligence.
        </p>
      </ScrollReveal>

      <ScrollReveal>
        <p className="text-lg mb-6">
          We believe in a future where AI amplifies rather than replaces human creativity, 
          where technology serves as a catalyst for artistic expression, and where the 
          boundaries between human and machine blur into a symphony of innovation.
        </p>
      </ScrollReveal>

      <ScrollReveal>
        <h2 className="text-2xl font-semibold mt-12 mb-6">Our Core Principles</h2>
        <div className="grid md:grid-cols-2 gap-8">
          <div className="bg-secondary/20 p-6 border border-primary">
            <h3 className="text-xl font-semibold mb-3">Symbiotic Growth</h3>
            <p>Fostering mutual evolution between human creativity and AI capabilities</p>
          </div>
          <div className="bg-secondary/20 p-6 border border-primary">
            <h3 className="text-xl font-semibold mb-3">Ethical Innovation</h3>
            <p>Advancing technology while preserving human agency and artistic integrity</p>
          </div>
          <div className="bg-secondary/20 p-6 border border-primary">
            <h3 className="text-xl font-semibold mb-3">Creative Liberation</h3>
            <p>Breaking free from traditional constraints through AI augmentation</p>
          </div>
          <div className="bg-secondary/20 p-6 border border-primary">
            <h3 className="text-xl font-semibold mb-3">Collective Evolution</h3>
            <p>Building a community of forward-thinking creators and innovators</p>
          </div>
        </div>
      </ScrollReveal>
    </div>
  );
}
