import React, { useEffect, useState } from 'react';
import './StarBackground.css';

const StarBackground = () => {
    const [shootingStars, setShootingStars] = useState([]);

    const generateRandomStar = () => {
        return {
            id: Date.now(),
            top: Math.random() * 100,
            left: 100, // Start from the right edge
            delay: Math.random() * 2
        };
    };

    useEffect(() => {
        // Create initial shooting stars
        const initialStars = Array.from({ length: 5 }, () => generateRandomStar());
        setShootingStars(initialStars);

        // Add new shooting stars periodically
        const interval = setInterval(() => {
            setShootingStars(prevStars => {
                const newStar = generateRandomStar();
                return [...prevStars.slice(-4), newStar];
            });
        }, 1500);

        return () => clearInterval(interval);
    }, []);

    return (
        <div className="star-background">
            <div className="stars"></div>
            {shootingStars.map(star => (
                <div
                    key={star.id}
                    className="shooting-star"
                    style={{
                        top: `${star.top}%`,
                        left: `${star.left}%`,
                        animationDelay: `${star.delay}s`
                    }}
                />
            ))}
        </div>
    );
};

export default StarBackground; 