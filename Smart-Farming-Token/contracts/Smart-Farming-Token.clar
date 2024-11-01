;; Title: Climate-Smart Farming Token (CSFT)
;; Description: A token system that rewards farmers for climate-friendly agricultural practices

;; -----------------------------
;; Constants
;; -----------------------------

;; Error codes
(define-constant ERR-NOT-AUTHORIZED (err u100))
(define-constant ERR-ALREADY-REGISTERED (err u101))
(define-constant ERR-NOT-REGISTERED (err u102))
(define-constant ERR-INVALID-PRACTICE (err u103))
(define-constant ERR-COOLDOWN-ACTIVE (err u104))

;; Practice IDs and their corresponding scores
(define-constant PRACTICE-NO-TILL u1)
(define-constant PRACTICE-COVER-CROP u2)
(define-constant PRACTICE-CROP-ROTATION u3)
(define-constant PRACTICE-ORGANIC u4)
(define-constant PRACTICE-WATER-EFFICIENT u5)
(define-constant PRACTICE-AGROFORESTRY u6)

;; Configuration
(define-constant CLAIM-COOLDOWN-PERIOD u1440) ;; ~10 days at 10 min block time
(define-constant TOKEN-REWARD-MULTIPLIER u100) ;; 100 tokens per score point

;; -----------------------------
;; Token Definition
;; -----------------------------
;; Define the fungible token
(define-fungible-token csft)


;; -----------------------------
;; Data Storage
;; -----------------------------
;; Contract owner
(define-data-var contract-owner principal ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM)

;; Farmer data storage
(define-map farmers
    principal  ;; farmer address
    {
        registered: bool,
        total-score: uint,
        last-claim: uint,
        practices: (list 6 uint)  ;; list of verified practices
    }
)

;; Verifier authorization
(define-map verifiers principal bool)

;; Practice definitions
(define-map practice-scores
    uint  ;; practice ID
    {
        name: (string-ascii 20),
        score: uint,
        active: bool
    }
)

;; -----------------------------
;; Authorization Functions
;; -----------------------------
;; Check if caller is contract owner
(define-private (is-contract-owner)
    (is-eq tx-sender (var-get contract-owner))
)

;; Check if caller is authorized verifier
(define-private (is-verifier (caller principal))
    (default-to false (map-get? verifiers caller))
)

;; -----------------------------
;; Administrative Functions
;; -----------------------------
;; Add a new verifier
(define-public (add-verifier (new-verifier principal))
    (begin
        (asserts! (is-contract-owner) ERR-NOT-AUTHORIZED)
        (map-set verifiers new-verifier true)
        (ok true)
    )
)

;; Remove a verifier
(define-public (remove-verifier (verifier principal))
    (begin
        (asserts! (is-contract-owner) ERR-NOT-AUTHORIZED)
        (map-delete verifiers verifier)
        (ok true)
    )
)

;; Initialize practice scores
(define-public (initialize-practices)
    (begin
        (asserts! (is-contract-owner) ERR-NOT-AUTHORIZED)
        (map-set practice-scores PRACTICE-NO-TILL 
            {name: "No-Till Farming", score: u10, active: true})
        (map-set practice-scores PRACTICE-COVER-CROP 
            {name: "Cover Cropping", score: u8, active: true})
        (map-set practice-scores PRACTICE-CROP-ROTATION 
            {name: "Crop Rotation", score: u6, active: true})
        (map-set practice-scores PRACTICE-ORGANIC 
            {name: "Organic Farming", score: u12, active: true})
        (map-set practice-scores PRACTICE-WATER-EFFICIENT 
            {name: "Water Efficient", score: u8, active: true})
        (map-set practice-scores PRACTICE-AGROFORESTRY 
            {name: "Agroforestry", score: u15, active: true})
        (ok true)
    )
)

