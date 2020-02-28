(import (ffi))

;; ffi bindings for assimp

;; Meshes

;; some constants

;; ---------------------------------------------------------------------------
;; Limits. These values are required to match the settings Assimp was
;; compiled against. Therefore, do not redefine them unless you build the
;; library from source using the same definitions.
;; ---------------------------------------------------------------------------

;; Maximum number of indices per face (polygon).
(define +max-face-indices+ #x7fff)

;; Maximum number of indices per face (polygon).
(define +max-bone-weights+ #x7fffffff)

;; Maximum number of vertices per mesh.
(define +max-vertices+ #x7fffffff)

;; Maximum number of faces per mesh.
(define +max-faces+ #x7fffffff)

;; Supported number of vertex color sets per mesh.
(define +max-number-of-color-sets+ #x8)

;; Supported number of texture coord sets (UV(W) channels) per mesh
(define +max-number-of-texture-coords+ #x8)

;; structs

(define-ftype ai-real double-float)

(define-foreign-struct vector3
  ((x ai-real)
   (y ai-real)
   (z ai-real)))

;; defines collection fns like
;; vector3-pointer-map to work on the array pointers
(define-collection-lambdas vector3)

(define-foreign-struct color4
  ((r ai-real)
   (g ai-real)
   (b ai-real)
   (a ai-real)))

(define-ftype aabb
  (struct (min vector3)
	  (max vector3)))

;; struct representing a geometry or model with a single material
;; A mesh uses only a single material which is referenced by a material ID
(define-foreign-struct mesh
  ((primitive-types  unsigned)
   (num-vertices  unsigned)
   (num-faces  unsigned)
   (vertices  (* vector3))
   (normals (* vector3))
   (tangents (* vector3))
   (bit-tangents (* vector3))
   (colors (array #x8 (* color4)))
   (texture-coords (array #x8 (* vector3)))
   (num-uv-component (array #x8 unsigned))
   (faces uptr)
   (num-bones unsigned)
   (bones uptr)
   (material-index unsigned)
   (name uptr)
   (num-anim-meshes unsigned)
   (anim-meshes uptr)
   (method unsigned)
   (_aabb aabb)))

(define-foreign-struct scene
  ((flags  unsigned-32)
   (root-node  uptr)
   (num-meshes  unsigned-32)
   (meshes  uptr)
   (num-materials  unsigned)
   (materials  uptr)
   (num-animations  unsigned)
   (animations  uptr)
   (num-textures  unsigned)
   (textures  uptr)
   (num-lights  unsigned)
   (lights  uptr)
   (num-cameras  unsigned)
   (cameras  uptr)
   (metadata  uptr)))

(define-enum-ftype ai-process-steps
  (calc-tangent-space #x1)
  (join-identical-vertices #x2)
  (make-left-handed #x4)
  (triangulate #x8)
  (remove-component #x10)
  (gen-normals #x20)
  (gen-smooth-normals #x40)
  (split-large-meshes #x80)
  (pretransform-vertices #x100)
  (limit-bone-weights #x200)
  (validate-data-structure #x400)
  (improve-cache-locality #x800)
  (remove-redundant-materials #x1000)
  (fix-infacing-normals #x2000)
  (sort-by-ptype #x8000)
  (find-degenerates #x10000)
  (find-invalid-data #x20000)
  (gen-UV-coords #x40000)
  (transform-UV-coords #x80000)
  (find-instances #x100000)
  (optimize-meshes #x200000)
  (optimize-graph #x400000)
  (flip-UVs #x800000)
  (flip-winding-order #x1000000)
  (split-by-bone-count #x2000000)
  (debone #x4000000))


;; load library

(load-shared-object "libassimp.so")

;; define native functions

(define import_file
  (foreign-procedure "aiImportFile" (string unsigned) (* scene)))


;; run

(define scene-ptr
  (import_file "../models/teapot.obj"
	       (bitwise-ior flip-winding-order triangulate pretransform-vertices)))

(define mesh-ptr
  (make-ftype-pointer mesh (foreign-ref 'uptr (scene-meshes scene-ptr) 0)))

(define vertices 
  (vector3-pointer-map (lambda (v) (list (vector3-x v)
				    (vector3-y v)
				    (vector3-z v)))
		       (make-array-pointer (mesh-num-vertices mesh-ptr)
					   (mesh-vertices mesh-ptr)
					   'vector3)))



