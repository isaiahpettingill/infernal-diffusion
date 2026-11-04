/* Adapted from Adobe TrustMark tm_watermark.js, Copyright 2024 Adobe.
 * MIT licensed; see LICENSE. Original preprocessing retained for Q compatibility. */
let ort, session_resize;
export function configureResize(runtime, session) { ort = runtime; session_resize = session; }
function computeScalesFixed(targetDims, inputDims) {
  const [batch, channels, height, width] = inputDims;
  const [targetHeight, targetWidth] = targetDims;

  function computeScale(originalSize, targetSize) {
    let minScale = targetSize / originalSize;
    let maxScale = (targetSize + 1) / originalSize;
    let scale;
    let adjustedSize;

    const tolerance = 1e-12;
    let iterations = 0;
    const maxIterations = 100;

    while (iterations < maxIterations) {
      scale = (minScale + maxScale) / 2;
      adjustedSize = Math.floor(originalSize * scale + tolerance);

      if (adjustedSize < targetSize) {
        minScale = scale;
      } else if (adjustedSize > targetSize) {
        maxScale = scale;
      } else {
        break; // Found the correct scale
      }

      iterations++;
    }

    return scale;
  }

  const scaleH = computeScale(height, targetHeight);
  const scaleW = computeScale(width, targetWidth);

  return new Float32Array([1.0, 1.0, scaleH, scaleW]);
}

/**
 * Resizes the image tensor to a square size suitable for watermark decoding.
 * @param {ort.Tensor} inputTensor - The input image tensor.
 * @param {number} targetSize - The target size for resizing.
 * @returns {Promise<ort.Tensor>} The resized tensor.
 */
async function runResizeModelSquare(inputTensor, targetSize, force_square) {
    try {
        const inputDims = inputTensor.dims;  // Get dimensions of the input tensor
        const [batch, channels, height, width] = inputDims;

        // Compute the aspect ratio
        const aspectRatio = width / height;
        const lscape= (aspectRatio>=1.0);

        let croppedTensor = inputTensor;
        let cropWidth = width;
        let cropHeight = height;

        // If the aspect ratio is greater than 2.0, we need to crop the center square
        if (lscape && (aspectRatio > 2.0 || force_square)) {
            cropWidth = height;  // Take a square from the width
            const offsetX = Math.floor((width - cropWidth) / 2);  // Horizontal center crop
            croppedTensor = await cropTensor(inputTensor, offsetX, 0, cropWidth, height);
        }

        if (!lscape && (aspectRatio < 0.5 || force_square)) {
            cropHeight = width;  // Take a square from the height
            const offsetY = Math.floor((height - cropHeight) / 2);  // Vertical center crop
            croppedTensor = await cropTensor(inputTensor, 0, offsetY, width, cropHeight);
        }

        // After cropping, resize the tensor to the target size
        const targetDims = [targetSize, targetSize];
        const scales = computeScalesFixed(targetDims, [batch, channels, cropHeight, cropWidth]);
        const scalesTensor = new ort.Tensor('float32', scales, [4]);

        // Prepare the target size tensor
        const targetSizeTensor = new ort.Tensor('int64', new BigInt64Array([BigInt(targetSize)]), [1]);

        // Set up the feeds for the model
        const feeds = {
            'X': croppedTensor,       // Cropped image tensor
            'scales': scalesTensor,   // Scales tensor
            'target_size': targetSizeTensor  // Dynamic target size tensor
        };

        try {
            const results = await session_resize.run(feeds);
            return results['Y'];
        } finally {
            scalesTensor.dispose();
            targetSizeTensor.dispose();
            if (croppedTensor !== inputTensor) croppedTensor.dispose();
        }

    } catch (error) {
        throw error;
    }
}

// Helper function to crop the tensor
async function cropTensor(inputTensor, offsetX, offsetY, cropWidth, cropHeight) {
    const [batch, channels, height, width] = inputTensor.dims;
    const croppedData = new Float32Array(batch * channels * cropWidth * cropHeight);
    const inputData = inputTensor.data;

    let k = 0;
    for (let c = 0; c < channels; c++) {
        for (let y = 0; y < cropHeight; y++) {
            for (let x = 0; x < cropWidth; x++) {
                const srcIndex = c * width * height + (y + offsetY) * width + (x + offsetX);
                croppedData[k++] = inputData[srcIndex];
            }
        }
    }

    return new ort.Tensor('float32', croppedData, [batch, channels, cropHeight, cropWidth]);
}



export { runResizeModelSquare };
