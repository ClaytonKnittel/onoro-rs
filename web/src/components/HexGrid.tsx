import React from 'react';

export interface Tile {
  x: number;
  y: number;
  component: React.ReactNode;
}

export default function HexGrid(props: {
  tileSpacing: string | number;
  tiles: Tile[];
}) {
  const [minX, maxX] = props.tiles
    .map(({ x }) => [x, x])
    .reduce(([minX, maxX], [x, _]) => [Math.min(x, minX), Math.max(x, maxX)]);
  const [minY, maxY] = props.tiles
    .map(({ y }) => [y, y])
    .reduce(([minY, maxY], [y, _]) => [Math.min(y, minY), Math.max(y, maxY)]);

  const width = maxX - minX + 1;
  const height = maxY - minY + 1;
  const components: React.ReactNode[][] = new Array(height)
    .fill(undefined)
    .map(() => new Array(width).fill(undefined).map(() => <></>));

  for (const tile of props.tiles) {
    components[tile.y - minY][tile.x - minX] = tile.component;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      {components.map((row, y) => (
        <div style={{ display: 'flex', flexDirection: 'row' }}>
          {row.map((tile, x) => (
            <div
              key={`${x},${y}`}
              style={{
                display: 'inline-block',
                width: props.tileSpacing,
                height: props.tileSpacing,
                border: '1px solid black',
              }}
            >
              {tile}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
