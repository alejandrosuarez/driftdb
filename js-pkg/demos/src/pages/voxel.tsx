import { DRIFTDB_URL } from '@/config'
import { OrbitControls } from '@react-three/drei'
import { Canvas, ThreeEvent } from '@react-three/fiber'
import { DriftDBProvider, usePresence, useSharedReducer } from 'driftdb-react'
import { useCallback, useRef, useState } from 'react'
import { Vector3, Vector3Tuple } from 'three'
import { CompactPicker } from 'react-color'

const DIM = 15

interface Voxel {
    position: Vector3Tuple
    color: any
    opacity: number
}

function Voxel(props: { voxel: Voxel, name?: string }) {
    const position: Vector3Tuple = [props.voxel.position[0], props.voxel.position[1] + 0.5, props.voxel.position[2]]
    return (
        <mesh
            name={props.name}
            position={position}
            scale={1}
        >
            <boxGeometry args={[1, 1, 1]} />

            <meshPhongMaterial color={props.voxel.color} opacity={props.voxel.opacity} transparent={props.voxel.opacity < 1} />
        </mesh>
    )
}

function getPosition(event: ThreeEvent<PointerEvent>): Vector3Tuple | null {
    if (event.intersections.length === 0) return null

    const { face, point } = event.intersections[0]
    const normal: Vector3 = face!.normal.clone()

    const pos: Vector3 = point.clone().add(new Vector3(0.5, 0.0, 0.5))

    const c = pos.add(normal.multiplyScalar(0.5)).floor()
    return c.toArray()
}

type VoxelAction = {
    type: 'add'
    voxel: Voxel
} | {
    type: 'remove'
    name: string
}

function voxelReducer(state: Record<string, Voxel>, action: VoxelAction): Record<string, Voxel> {
    if (action.type === 'add') {
        let key = action.voxel.position.join(':')

        return {
            ...state,
            [key]: action.voxel
        }
    }

    if (action.type === 'remove') {
        const newState = { ...state }
        delete newState[action.name]
        return newState
    }

    return state
}

interface VoxelSetProps {
    voxels: Record<string, Voxel>
}

function VoxelSet(props: VoxelSetProps) {
    return <>{
        Object.entries(props.voxels).map(([index, voxel]) => <Voxel key={index} voxel={voxel} name={index} />)
    }</>
}

export function VoxelEditor() {
    const [ghostPosition, setGhostPosition] = useState<[number, number, number] | null>(null)
    const [voxels, dispatch] = useSharedReducer("voxels", voxelReducer, [] as any)
    const [color, setColor] = useState('#D33115')
    
    const positionHasBeenSet = useRef(false)
    const setInitialCameraPosition = (controls: any) => {
        if (controls && !positionHasBeenSet.current) {
            controls.object.position.set(0, DIM, DIM)
            positionHasBeenSet.current = true
        }
    }

    const presence = usePresence("users", {
        position: ghostPosition,
        color: color
    })

    const pointerMove = useCallback((event: ThreeEvent<PointerEvent>) => {
        setGhostPosition(getPosition(event))
    }, [setGhostPosition])

    const handleClick = useCallback((event: ThreeEvent<MouseEvent>) => {
        if (event.delta > 5) {
            // ignore drag events, which are handled by the orbit control
            return
        }

        if (event.shiftKey) {
            if (event.object.name) {
                dispatch({
                    type: 'remove',
                    name: event.object.name
                })
            }

            event.stopPropagation()
            return
        }

        const position = getPosition(event as any)
        if (position) {
            dispatch({
                type: 'add',
                voxel: { position, color, opacity: 1 }
            })
        }
    }, [color, dispatch])

    return (
        <>
            <div style={{ position: 'absolute', top: 0, right: 0, left: 0, bottom: 0 }}>
                <Canvas>
                    <OrbitControls ref={setInitialCameraPosition} />
                    <ambientLight intensity={0.5} />
                    <spotLight position={[10, 10, 10]} angle={0.15} penumbra={1} />
                    <pointLight position={[-10, -10, -10]} />

                    {
                        ghostPosition ? <Voxel voxel={{ position: ghostPosition, color: 0x000000, opacity: 0.5 }} /> : null
                    }

                    {
                        Object.entries(presence).map(([id, user]) => {
                            if (user.value.position === null) return null

                            return <Voxel key={id} voxel={{ position: user.value.position, color: user.value.color, opacity: 0.5 }} />
                        })
                    }

                    <gridHelper args={[DIM, DIM]} position={[0, 0.001, 0]} />

                    <group onPointerMove={pointerMove} onClick={handleClick}>
                        <mesh
                            scale={1}
                            position={[0, -0.05, 0]}
                        >
                            <boxGeometry args={[DIM, 0.1, DIM]} />
                            <meshStandardMaterial color="#eee" opacity={1} />
                        </mesh>
                        <VoxelSet voxels={voxels} />
                    </group>

                </Canvas>
            </div>
            <CompactPicker color={color} onChangeComplete={(color) => setColor(color.hex)} />
        </>
    )
}

export default function App() {
    return (
        <DriftDBProvider api={DRIFTDB_URL}>
            <VoxelEditor />
        </DriftDBProvider>
    )
}
