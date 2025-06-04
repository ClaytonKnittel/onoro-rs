import React from 'react';

export default function Circle(props: {
  radius: string | number;
  style?: Omit<React.CSSProperties, 'borderRadius' | 'width' | 'height'>;
}) {
  return (
    <div
      style={{
        ...props.style,
        borderRadius: '50%',
        width: `calc(2 * ${props.radius})`,
        height: `calc(2 * ${props.radius})`,
      }}
    ></div>
  );
}
