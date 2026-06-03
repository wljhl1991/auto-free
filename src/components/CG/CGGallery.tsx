import { useState, useCallback } from 'react';
import styles from './CGGallery.module.css';

interface CGEntry {
  id: string;
  url: string;
  description: string;
}

interface CGGalleryProps {
  cgList: CGEntry[];
  onClose: () => void;
}

function CGGallery({ cgList, onClose }: CGGalleryProps) {
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);

  const handlePrev = useCallback(() => {
    if (lightboxIndex === null) return;
    setLightboxIndex((lightboxIndex - 1 + cgList.length) % cgList.length);
  }, [lightboxIndex, cgList.length]);

  const handleNext = useCallback(() => {
    if (lightboxIndex === null) return;
    setLightboxIndex((lightboxIndex + 1) % cgList.length);
  }, [lightboxIndex, cgList.length]);

  const handleCloseLightbox = useCallback(() => {
    setLightboxIndex(null);
  }, []);

  const currentCG = lightboxIndex !== null ? cgList[lightboxIndex] : null;

  return (
    <>
      <div className={styles.galleryOverlay} onClick={(e) => { e.stopPropagation(); onClose(); }}>
        <div className={styles.galleryPanel} onClick={(e) => e.stopPropagation()}>
          <div className={styles.galleryHeader}>
            <h3>CG 回廊</h3>
            <button className={styles.galleryCloseBtn} onClick={onClose}>✕</button>
          </div>
          <div className={styles.galleryBody}>
            {cgList.length === 0 ? (
              <div className={styles.galleryEmpty}>
                <span>尚未解锁任何 CG</span>
              </div>
            ) : (
              <div className={styles.galleryGrid}>
                {cgList.map((cg, index) => (
                  <div
                    key={cg.id}
                    className={styles.galleryItem}
                    onClick={() => setLightboxIndex(index)}
                  >
                    <img src={cg.url} alt={cg.description} />
                    <div className={styles.galleryItemDesc}>{cg.description}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {currentCG && (
        <div className={styles.lightbox} onClick={(e) => { e.stopPropagation(); handleCloseLightbox(); }}>
          <div className={styles.lightboxContent} onClick={(e) => e.stopPropagation()}>
            <span className={styles.lightboxCounter}>
              {lightboxIndex! + 1} / {cgList.length}
            </span>
            <button className={styles.lightboxClose} onClick={handleCloseLightbox}>✕</button>
            <img className={styles.lightboxImage} src={currentCG.url} alt={currentCG.description} />
            <div className={styles.lightboxDesc}>{currentCG.description}</div>
            {cgList.length > 1 && (
              <>
                <button className={`${styles.lightboxNav} ${styles.lightboxPrev}`} onClick={handlePrev}>◀</button>
                <button className={`${styles.lightboxNav} ${styles.lightboxNext}`} onClick={handleNext}>▶</button>
              </>
            )}
          </div>
        </div>
      )}
    </>
  );
}

export default CGGallery;
