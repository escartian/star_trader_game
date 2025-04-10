import React, { useEffect, useRef, useState } from 'react';
import './CanvasBackground.css';

interface Star {
    x: number;
    y: number;
    size: number;
    brightness: number;
    twinkleSpeed: number;
    twinkleOffset: number;
    color: string;
}

interface Comet {
    x: number;
    y: number;
    speed: number;
    angle: number;
    size: number;
    trail: { x: number; y: number; opacity: number; size: number }[];
    active: boolean;
    color: string;
}


interface CanvasBackgroundProps {
    show?: boolean;
}

const CanvasBackground: React.FC<CanvasBackgroundProps> = ({ show = true }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const [cometCount, setCometCount] = useState(2);
    const starsRef = useRef<Star[]>([]);
    const cometsRef = useRef<Comet[]>([]);
    const animationFrameRef = useRef<number | undefined>(undefined);

    // Initialize stars with scientifically accurate density
    const initStars = (width: number, height: number) => {
        const stars: Star[] = [];
        const starCount = Math.floor((width * height) / 2000);

        // Extended scientifically accurate star colors based on temperature and composition
        const colors = [
            // Blue sequence (hottest)
            '#a5beff', // O-type (40,000-50,000K) - Bright blue-white
            '#9db4ff', // O-type (35,000-40,000K) - Blue-white
            '#93a7ff', // B-type (25,000-35,000K) - Deep blue-white
            '#8b9fff', // B-type (20,000-25,000K) - Blue-white
            '#a5c0ff', // B-type (15,000-20,000K) - Light blue-white
            
            // White sequence
            '#cad7ff', // A-type (9,000-10,000K) - Blue-tinted white
            '#e8eeff', // A-type (8,000-9,000K) - Pure white
            '#f8f7ff', // F-type (7,000-8,000K) - Yellow-white
            
            // Yellow sequence
            '#fff4ea', // G-type (5,700-6,000K) - Yellow-white (Sun-like)
            '#fff2e6', // G-type (5,200-5,700K) - Light yellow
            
            // Orange sequence
            '#ffd7b5', // K-type (4,500-5,200K) - Light orange
            '#ffd2a1', // K-type (4,000-4,500K) - Orange
            '#ffcc8f', // K-type (3,700-4,000K) - Deep orange
            
            // Red sequence
            '#ffcc6f', // M-type (3,000-3,700K) - Orange-red
            '#ffc469', // M-type (2,700-3,000K) - Light red
            '#ffb56b', // M-type (2,400-2,700K) - Deep red

            // Special types
            '#ffefc1', // Carbon stars (2,400-3,000K) - Yellowish
            '#ffcb8f', // S-type stars (2,400-3,500K) - Orange-yellow
            '#ff9e80', // R Coronae Borealis (6,000-7,000K) - Reddish-orange
            
            // Binary star combinations (rarer)
            '#fff4ea80', // Binary G-type overlay
            '#ffd2a180', // Binary K-type overlay
            '#ffcc6f80'  // Binary M-type overlay
        ];

        // Distribution weights to match realistic stellar distribution
        const weights = [
            0.01, 0.01, 0.02, 0.02, 0.02, // Blue sequence (rare)
            0.05, 0.05, 0.10,             // White sequence
            0.15, 0.15,                   // Yellow sequence (common)
            0.15, 0.10, 0.05,             // Orange sequence
            0.05, 0.03, 0.02,             // Red sequence
            0.01, 0.01, 0.01,             // Special types (very rare)
            0.02, 0.02, 0.01              // Binary combinations (rare)
        ];

        // Generate cumulative weights for weighted random selection
        const cumWeights = weights.reduce((acc, w, i) => {
            acc[i] = (acc[i-1] || 0) + w;
            return acc;
        }, [] as number[]);

        for (let i = 0; i < starCount; i++) {
            // Weighted random selection
            const rand = Math.random();
            const colorIndex = cumWeights.findIndex(w => rand <= w);
            const color = colors[colorIndex];

            // Adjust size and brightness based on star type
            const isHot = colorIndex < 5;  // Blue sequence
            const isBinary = colorIndex > 18;  // Binary stars
            const sizeMultiplier = isHot ? 1.4 : (isBinary ? 1.2 : 1);
            const brightnessMultiplier = isHot ? 1.3 : (isBinary ? 1.1 : 1);

            stars.push({
                x: Math.random() * width,
                y: Math.random() * height,
                size: (Math.random() * 1.5 + 0.5) * sizeMultiplier,
                brightness: (Math.random() * 0.7 + 0.3) * brightnessMultiplier,
                twinkleSpeed: Math.random() * 0.03 + 0.02,
                twinkleOffset: Math.random() * Math.PI * 2,
                color
            });
        }
        return stars;
    };

    // Initialize comets with more dramatic colors
    const initComets = () => {
        const colors = [
            '#ffffff', // White (ice)
            '#4fc3f7', // Blue (ionized gas)
            '#81c784', // Green (cyanogen)
            '#ff9800', // Orange (dust)
            '#e91e63', // Red (organic compounds)
            '#00bcd4', // Cyan (ionized gas)
            '#ff5722', // Deep Orange (dust)
            '#9c27b0'  // Purple (organic compounds)
        ];
        return Array(2).fill(null).map(() => ({
            x: -100,
            y: 0,
            speed: 0,
            angle: 0,
            size: 0,
            trail: [],
            active: false,
            color: colors[Math.floor(Math.random() * colors.length)]
        }));
    };

    // Generate a new comet with edge spawning
    const generateComet = (width: number, height: number): Comet => {
        const edge = Math.floor(Math.random() * 4); // 0: top, 1: right, 2: bottom, 3: left
        let x, y, angle;

        switch (edge) {
            case 0: // Top
                x = Math.random() * width;
                y = -50;
                angle = Math.random() * Math.PI + Math.PI / 4; // 45 to 225 degrees
                break;
            case 1: // Right
                x = width + 50;
                y = Math.random() * height;
                angle = Math.random() * Math.PI + Math.PI / 2; // 90 to 270 degrees
                break;
            case 2: // Bottom
                x = Math.random() * width;
                y = height + 50;
                angle = Math.random() * Math.PI - Math.PI / 4; // -45 to 135 degrees
                break;
            case 3: // Left
                x = -50;
                y = Math.random() * height;
                angle = Math.random() * Math.PI; // 0 to 180 degrees
                break;
            default:
                x = 0;
                y = 0;
                angle = 0;
        }

        const speed = Math.random() * 2 + 3;
        const colors = ['#ffffff', '#4fc3f7', '#81c784']; // White, Blue, Green comets
        const color = colors[Math.floor(Math.random() * colors.length)];

        return {
            x,
            y,
            speed,
            angle,
            size: Math.random() * 1 + 1,
            trail: [],
            active: true,
            color
        };
    };

    // Draw the background gradient
    const drawBackground = (ctx: CanvasRenderingContext2D, width: number, height: number) => {
        const gradient = ctx.createLinearGradient(0, 0, 0, height);
        gradient.addColorStop(0, '#000000');
        gradient.addColorStop(0.2, '#020205');
        gradient.addColorStop(0.4, '#030308');
        gradient.addColorStop(0.6, '#04040a');
        gradient.addColorStop(0.8, '#05050c');
        gradient.addColorStop(1, '#06060e');

        ctx.fillStyle = gradient;
        ctx.fillRect(0, 0, width, height);
    };

    // Draw stars with enhanced twinkling
    const drawStars = (ctx: CanvasRenderingContext2D, time: number) => {
        starsRef.current.forEach(star => {
            // More dramatic twinkling with multiple sine waves
            const twinkle1 = Math.sin(time * star.twinkleSpeed + star.twinkleOffset) * 0.4;
            const twinkle2 = Math.sin(time * star.twinkleSpeed * 1.7 + star.twinkleOffset) * 0.3;
            const twinkle3 = Math.sin(time * star.twinkleSpeed * 0.5 + star.twinkleOffset) * 0.2;
            const brightness = star.brightness + twinkle1 + twinkle2 + twinkle3;
            
            ctx.beginPath();
            ctx.arc(star.x, star.y, star.size, 0, Math.PI * 2);
            
            // Enhanced glow effect with multiple layers
            const gradient = ctx.createRadialGradient(
                star.x, star.y, 0,
                star.x, star.y, star.size * 4
            );
            gradient.addColorStop(0, star.color);
            gradient.addColorStop(0.3, `${star.color}80`);
            gradient.addColorStop(0.6, `${star.color}40`);
            gradient.addColorStop(1, 'rgba(255, 255, 255, 0)');
            
            ctx.fillStyle = gradient;
            ctx.fill();
        });
    };

    // Draw comets with particle-based trail
    const drawComets = (ctx: CanvasRenderingContext2D, width: number, height: number) => {
        cometsRef.current.forEach(comet => {
            if (!comet.active) return;

            // Update comet position
            comet.x += comet.speed * Math.cos(comet.angle);
            comet.y += comet.speed * Math.sin(comet.angle);

            // Add new particles to trail with varying sizes
            for (let i = 0; i < 3; i++) {
                const spread = 0.1;
                const particleAngle = comet.angle + (Math.random() - 0.5) * spread;
                const distance = Math.random() * 2;
                
                // Smaller particles with less variation
                const sizeMultiplier = 1 - (i * 0.15);
                const baseSize = Math.random() * 1 + 0.5;
                
                // Fix trail direction by adding particles behind the comet
                comet.trail.push({
                    x: comet.x - Math.cos(comet.angle) * distance * 2,
                    y: comet.y - Math.sin(comet.angle) * distance * 2,
                    opacity: 0.8,
                    size: baseSize * sizeMultiplier
                });
            }

            // Update trail particles with faster fade
            comet.trail.forEach(particle => {
                particle.opacity -= 0.01;
                particle.size *= 0.99;
            });

            // Remove old particles
            comet.trail = comet.trail.filter(particle => particle.opacity > 0);

            // Draw trail particles with reduced glow
            comet.trail.forEach(particle => {
                ctx.beginPath();
                ctx.arc(particle.x, particle.y, particle.size, 0, Math.PI * 2);
                
                // Create particle glow with reduced intensity
                const gradient = ctx.createRadialGradient(
                    particle.x, particle.y, 0,
                    particle.x, particle.y, particle.size * 2
                );
                gradient.addColorStop(0, `${comet.color}${Math.floor(particle.opacity * 200).toString(16).padStart(2, '0')}`);
                gradient.addColorStop(0.5, `${comet.color}${Math.floor(particle.opacity * 100).toString(16).padStart(2, '0')}`);
                gradient.addColorStop(1, 'rgba(255, 255, 255, 0)');
                
                ctx.fillStyle = gradient;
                ctx.fill();
            });

            // Draw comet head with reduced glow
            const headGradient = ctx.createRadialGradient(
                comet.x, comet.y, 0,
                comet.x, comet.y, comet.size * 3
            );
            headGradient.addColorStop(0, comet.color);
            headGradient.addColorStop(0.3, `${comet.color}60`);
            headGradient.addColorStop(0.6, `${comet.color}20`);
            headGradient.addColorStop(1, 'rgba(255, 255, 255, 0)');

            ctx.beginPath();
            ctx.arc(comet.x, comet.y, comet.size, 0, Math.PI * 2);
            ctx.fillStyle = headGradient;
            ctx.fill();

            // Check if comet is out of bounds
            if (comet.x < -100 || comet.x > width + 100 || comet.y < -100 || comet.y > height + 100) {
                comet.active = false;
            }
        });
    };

    // Animation loop
    const animate = (time: number) => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const ctx = canvas.getContext('2d');
        if (!ctx) return;

        const width = canvas.width;
        const height = canvas.height;

        // Clear canvas
        ctx.clearRect(0, 0, width, height);

        // Draw background
        drawBackground(ctx, width, height);

        // Draw stars
        drawStars(ctx, time / 1000);

        // Draw comets
        drawComets(ctx, width, height);

        // Generate new comets if needed
        const activeComets = cometsRef.current.filter(comet => comet.active).length;
        if (activeComets < 2 && Math.random() < 0.01) { // 1% chance each frame
            const inactiveIndex = cometsRef.current.findIndex(comet => !comet.active);
            if (inactiveIndex !== -1) {
                cometsRef.current[inactiveIndex] = generateComet(width, height);
            }
        }

        animationFrameRef.current = requestAnimationFrame(animate);
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        // Set initial canvas size
        const setInitialCanvasSize = () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        };

        // Set canvas size and handle resizing
        const resizeCanvas = () => {
            const canvas = canvasRef.current;
            if (!canvas) return;

            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;

            // Reinitialize stars with the same enhanced color distribution
            starsRef.current = initStars(canvas.width, canvas.height);
        };

        // Set initial size and generate stars
        setInitialCanvasSize();
        starsRef.current = initStars(canvas.width, canvas.height);
        cometsRef.current = initComets();

        // Start animation
        animationFrameRef.current = requestAnimationFrame(animate);

        // Handle window resize
        window.addEventListener('resize', resizeCanvas);

        // Cleanup
        return () => {
            window.removeEventListener('resize', resizeCanvas);
            if (animationFrameRef.current) {
                cancelAnimationFrame(animationFrameRef.current);
            }
        };
    }, []);

    useEffect(() => {
        const handleCometCountUpdate = (event: CustomEvent) => {
            setCometCount(event.detail.count);
        };

        window.addEventListener('updateCometCount', handleCometCountUpdate as EventListener);

        return () => {
            window.removeEventListener('updateCometCount', handleCometCountUpdate as EventListener);
        };
    }, []);

    return show ? (
        <canvas
            ref={canvasRef}
            className="canvas-background"
            style={{
                position: 'fixed',
                top: 0,
                left: 0,
                width: '100%',
                height: '100%',
                zIndex: 0
            }}
        />
    ) : null;
};

export default CanvasBackground; 