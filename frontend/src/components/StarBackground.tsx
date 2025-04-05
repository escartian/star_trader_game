import React, { useEffect, useState, useRef } from 'react';
import './StarBackground.css';

interface Star {
    id: string;
    top: number;
    left: number;
    twinkleDuration: number;
    twinkleDelay: number;
}

interface ShootingStar {
    id: number;
    startX: number;
    startY: number;
    moveX: number;
    moveY: number;
    size: number;
    duration: number;
    delay: number;
}

const StarBackground: React.FC = () => {
    const [stars, setStars] = useState<Star[]>([]);
    const [shootingStars, setShootingStars] = useState<ShootingStar[]>([]);
    const starCounter = useRef(0);
    const starCreationTime = useRef<{ [key: string]: number }>({});

    const generateRandomStar = (): Star => {
        starCounter.current += 1;
        return {
            id: `star-${starCounter.current}`,
            top: Math.random() * 100,
            left: Math.random() * 100,
            twinkleDuration: 2 + Math.random() * 3, // Random duration between 2-5 seconds
            twinkleDelay: Math.random() * 2 // Random delay between 0-2 seconds
        };
    };

    const generateShootingStar = (): ShootingStar => {
        const startFromLeft = Math.random() > 0.5;
        const startY = Math.random() * 100;
        const size = Math.random() * 4 + 2; // Smaller size for comets
        const duration = 4; // Faster movement
        const delay = Math.random() * 15; // Much less frequent appearance

        return {
            id: Date.now() + Math.random(),
            startX: startFromLeft ? -5 : 105, // Start slightly outside the viewport
            startY,
            moveX: startFromLeft ? 110 : -110, // Move across the entire viewport
            moveY: 0, // No vertical movement
            size,
            duration,
            delay
        };
    };

    useEffect(() => {
        console.log('Initializing StarBackground component');
        
        // Create initial stars
        const initialStars = Array.from({ length: 100 }, () => generateRandomStar());
        setStars(initialStars);

        // Create initial shooting stars with staggered delays
        const initialShootingStars = Array.from({ length: 2 }, (_, index) => {
            const star = generateShootingStar();
            starCreationTime.current[star.id.toString()] = Date.now();
            return {
                ...star,
                delay: index * 5 // Longer stagger between initial stars
            };
        });
        console.log('Created initial shooting stars:', initialShootingStars);
        setShootingStars(initialShootingStars);

        // Add new shooting stars periodically
        const interval = setInterval(() => {
            setShootingStars(prevStars => {
                // Remove stars that have completed their animation (after 12 seconds)
                const activeStars = prevStars.filter(star => {
                    const creationTime = starCreationTime.current[star.id.toString()];
                    const starAge = Date.now() - creationTime;
                    return starAge < 12000; // Keep stars for 12 seconds
                });
                
                // Only add a new star if we have less than 2 active stars
                if (activeStars.length < 2) {
                    const newStar = generateShootingStar();
                    starCreationTime.current[newStar.id.toString()] = Date.now();
                    return [...activeStars, newStar];
                }
                return activeStars;
            });
        }, 8000); // Check every 8 seconds instead of 4

        return () => {
            console.log('Cleaning up StarBackground component');
            clearInterval(interval);
        };
    }, []);

    return (
        <div className="star-background">
            {stars.map(star => (
                <div
                    key={star.id}
                    className="star"
                    style={{
                        top: `${star.top}%`,
                        left: `${star.left}%`,
                        '--twinkle-duration': `${star.twinkleDuration}s`,
                        '--twinkle-delay': `${star.twinkleDelay}s`
                    } as React.CSSProperties}
                />
            ))}
            {shootingStars.map(star => {
                const style = {
                    top: `${star.startY}%`,
                    left: `${star.startX}%`,
                    animationDelay: `${star.delay}s`,
                    '--move-x': `${star.moveX}`,
                    '--move-y': `${star.moveY}`,
                    '--size': `${star.size}px`
                } as React.CSSProperties;

                return (
                    <div key={star.id} className="shooting-star-container" style={style}>
                        <div className="shooting-star"></div>
                    </div>
                );
            })}
        </div>
    );
};

export default StarBackground; 