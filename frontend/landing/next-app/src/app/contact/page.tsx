'use client';

import { useState } from 'react';
import ScrollReveal from '../components/ScrollReveal';

export default function Contact() {
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [message, setMessage] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const subject = encodeURIComponent('New Inquiry from ' + name);
    const body = encodeURIComponent(`Name: ${name}\nEmail: ${email}\nMessage: ${message}`);
    window.location.href = `mailto:admin@author.works?subject=${subject}&body=${body}`;
  };

  return (
    <div className="max-w-2xl mx-auto space-y-8">
      <ScrollReveal>
        <h1 className="text-4xl font-bold mb-6">Join the Revolution</h1>
        <p className="text-lg mb-8">
          Ready to explore the frontiers of human-AI creativity? Connect with us 
          and become part of the next chapter in digital revolution.
        </p>
      </ScrollReveal>

      <ScrollReveal>
        <form className="space-y-6" onSubmit={handleSubmit}>
          <div>
            <label htmlFor="name" className="block text-sm font-medium mb-2">Name</label>
            <input 
              type="text" 
              id="name" 
              name="name" 
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-4 py-2 bg-secondary/20 border border-primary text-text focus:ring-1 focus:ring-accent outline-none" 
              required 
            />
          </div>
          <div>
            <label htmlFor="email" className="block text-sm font-medium mb-2">Email</label>
            <input 
              type="email" 
              id="email" 
              name="email" 
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full px-4 py-2 bg-secondary/20 border border-primary text-text focus:ring-1 focus:ring-accent outline-none" 
              required 
            />
          </div>
          <div>
            <label htmlFor="message" className="block text-sm font-medium mb-2">Message</label>
            <textarea 
              id="message" 
              name="message" 
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={4} 
              className="w-full px-4 py-2 bg-secondary/20 border border-primary text-text focus:ring-1 focus:ring-accent outline-none" 
              required
            ></textarea>
          </div>
          <button type="submit" className="btn-primary w-full">Initiate Contact</button>
        </form>
      </ScrollReveal>
    </div>
  );
}
