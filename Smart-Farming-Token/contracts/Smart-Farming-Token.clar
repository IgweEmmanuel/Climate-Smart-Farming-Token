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


;; -----------------------------
;; Farmer Registration
;; -----------------------------
;; Register a new farmer
(define-public (register-farmer)
    (let
        ((farmer-data (map-get? farmers tx-sender)))
        (asserts! (is-none farmer-data) ERR-ALREADY-REGISTERED)
        (map-set farmers tx-sender {
            registered: true,
            total-score: u0,
            last-claim: u0,
            practices: (list)
        })
        (ok true)
    )
)

;; -----------------------------
;; Practice Verification
;; -----------------------------
;; Verify a sustainable farming practice
(define-public (verify-practice (farmer principal) (practice-id uint))
    (let
        ((farmer-data (unwrap! (map-get? farmers farmer) ERR-NOT-REGISTERED))
         (practice (unwrap! (map-get? practice-scores practice-id) ERR-INVALID-PRACTICE)))
        (begin
            (asserts! (is-verifier tx-sender) ERR-NOT-AUTHORIZED)
            (asserts! (get active practice) ERR-INVALID-PRACTICE)
            
            ;; Update farmer's practices and score
            (map-set farmers farmer
                (merge farmer-data
                    {
                        practices: (unwrap! (as-max-len? 
                            (append (get practices farmer-data) practice-id) u6) 
                            ERR-NOT-AUTHORIZED),
                        total-score: (+ (get total-score farmer-data) (get score practice))
                    }
                )
            )
            (ok true)
        )
    )
)

;; -----------------------------
;; Token Rewards
;; -----------------------------
;; Claim token rewards based on sustainable practices
(define-public (claim-rewards)
    (let
        ((farmer-data (unwrap! (map-get? farmers tx-sender) ERR-NOT-REGISTERED))
         (current-block (unwrap-panic (get-block-info? time u0)))
         (cooldown-passed (> (- current-block (get last-claim farmer-data)) CLAIM-COOLDOWN-PERIOD))
         (reward-amount (* (get total-score farmer-data) TOKEN-REWARD-MULTIPLIER)))
        (begin
            (asserts! cooldown-passed ERR-COOLDOWN-ACTIVE)
            
            ;; Mint tokens based on score
            (try! (mint-csft tx-sender reward-amount))
            
            ;; Update last claim time
            (map-set farmers tx-sender
                (merge farmer-data {last-claim: current-block}))
            
            (ok reward-amount)
        )
    )
)

;; -----------------------------
;; Token Operations
;; -----------------------------
;; Mint tokens
(define-private (mint-csft (recipient principal) (amount uint))
    (ft-mint? csft amount recipient)
)

;; -----------------------------
;; Test Helper Functions
;; -----------------------------
;; Initialize test principals
(define-constant test-farmer ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant test-verifier ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG)

;; Test initialization function
(define-public (initialize-test)
    (begin
        ;; Add test verifier
        (try! (add-verifier test-verifier))
        ;; Initialize practices
        (try! (initialize-practices))
        ;; Register test farmer
        (contract-call? 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.climate-smart-farming register-farmer)
        (ok true)
    )
)

;; -----------------------------
;; Getter Functions
;; -----------------------------
;; Get farmer data
(define-read-only (get-farmer-data (farmer principal))
    (map-get? farmers farmer)
)

;; Get practice score details
(define-read-only (get-practice-details (practice-id uint))
    (map-get? practice-scores practice-id)
)

;; Check if address is verifier
(define-read-only (is-verifier? (address principal))
    (default-to false (map-get? verifiers address))
)

