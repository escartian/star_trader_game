import React, { useEffect, useRef } from 'react';
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

const CanvasBackground: React.FC = () => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const starsRef = useRef<Star[]>([]);
    const cometsRef = useRef<Comet[]>([]);
    const animationFrameRef = useRef<number | undefined>(undefined);

    // Initialize stars with scientifically accurate colors
    const initStars = (width: number, height: number) => {
        const stars: Star[] = [];
        const starCount = Math.floor((width * height) / 2000);

        // Scientifically accurate star colors based on temperature (Kelvin)
        const colors = [
            '#9db4ff', // O-type (30,000-50,000K) - Blue-white
            '#aabfff', // B-type (10,000-30,000K) - Blue-white
            '#cad7ff', // A-type (7,500-10,000K) - White
            '#f8f7ff', // F-type (6,000-7,500K) - Yellow-white
            '#fff4ea', // G-type (5,200-6,000K) - Yellow (Sun-like)
            '#ffd2a1', // K-type (3,700-5,200K) - Orange
            '#ffcc6f'  // M-type (2,400-3,700K) - Red
        ];

        for (let i = 0; i < starCount; i++) {
            const color = colors[Math.floor(Math.random() * colors.length)];
            stars.push({
                x: Math.random() * width,
                y: Math.random() * height,
                size: Math.random() * 1.5 + 0.5,
                brightness: Math.random() * 0.7 + 0.3,
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
            const oldWidth = canvas.width;
            const oldHeight = canvas.height;
            
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;

            // Scientifically accurate star colors based on temperature (Kelvin)
            const colors = [
                '#9db4ff', // O-type (30,000-50,000K) - Blue-white
                '#aabfff', // B-type (10,000-30,000K) - Blue-white
                '#cad7ff', // A-type (7,500-10,000K) - White
                '#f8f7ff', // F-type (6,000-7,500K) - Yellow-white
                '#fff4ea', // G-type (5,200-6,000K) - Yellow (Sun-like)
                '#ffd2a1', // K-type (3,700-5,200K) - Orange
                '#ffcc6f'  // M-type (2,400-3,700K) - Red
            ];

            // Calculate new star count needed with higher density
            const newWidth = canvas.width;
            const newHeight = canvas.height;
            const newStarCount = Math.floor((newWidth * newHeight) / 1000);
            const currentStarCount = starsRef.current.length;

            if (newStarCount > currentStarCount) {
                // Calculate the expanded areas
                const widthExpanded = newWidth > oldWidth;
                const heightExpanded = newHeight > oldHeight;
                
                // Add new stars to fill the expanded space
                const starsToAdd = newStarCount - currentStarCount;
                const newStars = Array(starsToAdd).fill(null).map(() => {
                    const color = colors[Math.floor(Math.random() * colors.length)];
                    
                    // Determine where to place the new star
                    let x, y;
                    if (widthExpanded && heightExpanded) {
                        // If both dimensions expanded, place star in the new corner areas
                        if (Math.random() < 0.5) {
                            x = Math.random() * (newWidth - oldWidth) + (newWidth > oldWidth ? oldWidth : 0);
                            y = Math.random() * newHeight;
                        } else {
                            x = Math.random() * newWidth;
                            y = Math.random() * (newHeight - oldHeight) + (newHeight > oldHeight ? oldHeight : 0);
                        }
                    } else if (widthExpanded) {
                        // If only width expanded, place star in the new side areas
                        x = Math.random() * (newWidth - oldWidth) + (newWidth > oldWidth ? oldWidth : 0);
                        y = Math.random() * newHeight;
                    } else if (heightExpanded) {
                        // If only height expanded, place star in the new top/bottom areas
                        x = Math.random() * newWidth;
                        y = Math.random() * (newHeight - oldHeight) + (newHeight > oldHeight ? oldHeight : 0);
                    } else {
                        // If no expansion, distribute evenly (shouldn't happen)
                        x = Math.random() * newWidth;
                        y = Math.random() * newHeight;
                    }

                    return {
                        x,
                        y,
                        size: Math.random() * 1.5 + 0.5,
                        brightness: Math.random() * 0.7 + 0.3,
                        twinkleSpeed: Math.random() * 0.03 + 0.02,
                        twinkleOffset: Math.random() * Math.PI * 2,
                        color
                    };
                });
                starsRef.current = [...starsRef.current, ...newStars];
            } else if (newStarCount < currentStarCount) {
                // Remove excess stars if canvas got smaller
                starsRef.current = starsRef.current.slice(0, newStarCount);
            }

            // Adjust existing stars' positions if they're now outside the canvas
            starsRef.current = starsRef.current.map(star => {
                if (star.x > newWidth || star.y > newHeight) {
                    return {
                        ...star,
                        x: Math.random() * newWidth,
                        y: Math.random() * newHeight
                    };
                }
                return star;
            });
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

    return (
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
    );
};

export default CanvasBackground; 