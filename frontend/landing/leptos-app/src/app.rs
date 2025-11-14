use leptos::*;
use leptos_meta::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let handle_nav = move |ev: ev::MouseEvent, id: &str| {
        ev.prevent_default();
        if let Some(element) = document().get_element_by_id(id) {
            let options = web_sys::ScrollIntoViewOptions::new();
            options.set_behavior(web_sys::ScrollBehavior::Smooth);
            let _ = element.scroll_into_view_with_scroll_into_view_options(&options);
        }
    };

    view! {
        <head>
            <Stylesheet id="leptos" href="/pkg/leptos-app.css"/>
            <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
            <Meta name="theme-color" content="#000000"/>
            <title>"author.works"</title>
        </head>
        
        <div class="terminal-container">
            <header class="terminal-header">
                <div class="terminal-nav">
                    <h1 class="text-2xl font-bold">
                        <a href="#" on:click=move |ev| handle_nav(ev, "home")>"author.works"</a>
                    </h1>
                    <nav>
                        <ul class="flex gap-8">
                            <li><a href="#" on:click=move |ev| handle_nav(ev, "home")>"Home"</a></li>
                            <li><a href="#" on:click=move |ev| handle_nav(ev, "about")>"About"</a></li>
                            <li><a href="#" on:click=move |ev| handle_nav(ev, "roadmap")>"Roadmap"</a></li>
                            <li><a href="#" on:click=move |ev| handle_nav(ev, "contact")>"Contact"</a></li>
                        </ul>
                    </nav>
                </div>
            </header>
            <main>
                <div id="home"><HomePage/></div>
                <div id="about"><AboutPage/></div>
                <div id="roadmap"><RoadmapPage/></div>
                <div id="contact"><ContactPage/></div>
            </main>
        </div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="content-container text-center">
            <h1 class="matrix-fade-in">"author.works"</h1>
            <p class="matrix-fade-in text-xl mb-12" style="animation-delay: 0.2s">
                "Where human imagination meets artificial intelligence, creating 
                a renaissance of boundless creativity and artistic expression."
            </p>
            <div class="grid">
                <div class="terminal-box">
                    <h2>"Neural Synthesis"</h2>
                    <p>
                        "Bridging the gap between human intuition and machine precision, 
                        creating art that transcends traditional boundaries."
                    </p>
                </div>
                <div class="terminal-box">
                    <h2>"Creative Amplification"</h2>
                    <p>
                        "Augmenting human creativity with AI assistance, enabling artists 
                        to explore new dimensions of expression."
                    </p>
                </div>
                <div class="terminal-box">
                    <h2>"Infinite Canvas"</h2>
                    <p>
                        "Unleashing limitless possibilities through the seamless fusion 
                        of human artistry and computational innovation."
                    </p>
                </div>
                <div class="terminal-box">
                    <h2>"Digital Renaissance"</h2>
                    <p>
                        "Ushering in a new era where technology and creativity converge, 
                        empowering artists to push the boundaries of imagination."
                    </p>
                </div>
            </div>
        </div>
    }
}

#[component]
fn AboutPage() -> impl IntoView {
    view! {
        <div class="content-container text-center space-y-12">
            <div class="matrix-fade-in">
                <h1 class="text-3xl font-bold mb-6">"Pioneering Digital Creation"</h1>
                <p class="text-xl mb-8">
                    "At author.works, we're architecting a future where human creativity 
                    and artificial intelligence dance in perfect harmony. Our mission is to unlock 
                    the boundless potential that exists at the intersection of human intuition 
                    and computational intelligence."
                </p>
            </div>

            <div class="matrix-fade-in" style="animation-delay: 0.2s">
                <p class="text-xl mb-12">
                    "We believe in a future where AI amplifies rather than replaces human creativity, 
                    where technology serves as a catalyst for artistic expression, and where the 
                    boundaries between human and machine blur into a symphony of innovation."
                </p>
            </div>

            <div class="matrix-fade-in" style="animation-delay: 0.4s">
                <h2 class="text-2xl font-semibold mb-8">"Our Core Principles"</h2>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-8">
                    <div class="terminal-box">
                        <h3 class="text-xl font-semibold mb-4">"Symbiotic Growth"</h3>
                        <p>"Fostering mutual evolution between human creativity and AI capabilities"</p>
                    </div>
                    <div class="terminal-box">
                        <h3 class="text-xl font-semibold mb-4">"Ethical Innovation"</h3>
                        <p>"Advancing technology while preserving human agency and artistic integrity"</p>
                    </div>
                    <div class="terminal-box">
                        <h3 class="text-xl font-semibold mb-4">"Creative Liberation"</h3>
                        <p>"Breaking free from traditional constraints through AI augmentation"</p>
                    </div>
                    <div class="terminal-box">
                        <h3 class="text-xl font-semibold mb-4">"Collective Evolution"</h3>
                        <p>"Building a community of forward-thinking creators and innovators"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn ContactPage() -> impl IntoView {
    let handle_contact = move |ev: ev::MouseEvent| {
        ev.prevent_default();
        let mail_to = "mailto:lpask001@gmail.com?subject=Author.Works%20Inquiry";
        window().location().set_href(mail_to).unwrap();
    };

    view! {
        <div class="content-container text-center max-w-2xl mx-auto space-y-8">
            <h1 class="text-3xl font-bold mb-6">"Join Our Creative Revolution"</h1>
            <p class="text-xl mb-8">
                "At the intersection of human ingenuity and artificial intelligence, 
                we're crafting the future of creative expression. Our mission is to 
                empower artists, writers, and visionaries with tools that amplify 
                their creative potential while preserving the essence of human 
                imagination."
            </p>
            <p class="text-xl mb-12">
                "Whether you're an artist seeking to explore new dimensions of creativity, 
                or a visionary interested in shaping the future of digital expression, 
                we invite you to join us on this extraordinary journey."
            </p>
            <div class="text-center">
                <button 
                    on:click=handle_contact
                    class="btn-primary px-8 py-3"
                >
                    "Connect With Us"
                </button>
            </div>
        </div>
    }
}

#[component]
fn RoadmapPage() -> impl IntoView {
    view! {
        <div class="content-container text-center prose max-w-none">
            <h1 class="text-3xl font-bold mb-8">Roadmap</h1>
            <div class="space-y-16">
                <section class="matrix-fade-in">
                    <h2 class="text-2xl font-semibold mb-4">Phase 1: Foundation</h2>
                    <ul class="list-disc pl-6 space-y-2">
                        <li>Launch initial platform architecture</li>
                        <li>Establish core AI integration framework</li>
                        <li>Develop basic collaborative tools</li>
                    </ul>
                </section>

                <section class="matrix-fade-in" style="animation-delay: 0.2s">
                    <h2 class="text-2xl font-semibold mb-4">Phase 2: Enhancement</h2>
                    <ul class="list-disc pl-6 space-y-2">
                        <li>Implement advanced AI capabilities</li>
                        <li>Expand creative toolset</li>
                        <li>Introduce collaborative features</li>
                    </ul>
                </section>

                <section class="matrix-fade-in" style="animation-delay: 0.4s">
                    <h2 class="text-2xl font-semibold mb-4">Phase 3: Innovation</h2>
                    <ul class="list-disc pl-6 space-y-2">
                        <li>Deploy cutting-edge AI models</li>
                        <li>Launch advanced project management tools</li>
                        <li>Integrate real-time collaboration features</li>
                    </ul>
                </section>

                <section class="matrix-fade-in" style="animation-delay: 0.6s">
                    <h2 class="text-2xl font-semibold mb-4">Future Vision</h2>
                    <ul class="list-disc pl-6 space-y-2">
                        <li>Explore emerging AI technologies</li>
                        <li>Expand platform capabilities</li>
                        <li>Enhance user experience and interface</li>
                    </ul>
                </section>
            </div>
        </div>
    }
}
