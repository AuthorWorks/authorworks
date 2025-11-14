import Link from 'next/link';
import ScrollReveal from './components/ScrollReveal';

export default function Home() {
  return (
    <div className="space-y-16">
      <section className="text-center py-20">
        <ScrollReveal>
          <h1 className="text-5xl font-bold mb-6">author.works</h1>
          <p className="text-xl mb-8 max-w-3xl mx-auto">
            Where human imagination meets artificial intelligence, creating a renaissance 
            of boundless creativity and artistic expression.
          </p>
          <Link href="/about" className="btn-primary">Enter the Future</Link>
        </ScrollReveal>
      </section>

      <section className="grid md:grid-cols-3 gap-12">
        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Neural Synthesis</h2>
          <p>Bridging the gap between human intuition and machine precision, 
             creating art that transcends traditional boundaries.</p>
        </ScrollReveal>
        
        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Creative Amplification</h2>
          <p>Augmenting human creativity with AI assistance, enabling artists 
             to explore new dimensions of expression.</p>
        </ScrollReveal>
        
        <ScrollReveal className="bg-secondary/20 p-8 border border-primary">
          <h2 className="text-2xl font-semibold mb-4">Infinite Canvas</h2>
          <p>Unleashing limitless possibilities through the seamless fusion 
             of human artistry and computational innovation.</p>
        </ScrollReveal>
      </section>

      <section className="text-center py-12">
        <ScrollReveal>
          <p className="text-xl italic">
            &ldquo;In the convergence of human and artificial intelligence, 
            we find not our replacement, but our renaissance.&rdquo;
          </p>
        </ScrollReveal>
      </section>
    </div>
  );
}
