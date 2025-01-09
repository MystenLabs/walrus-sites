// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import React, { useState } from "react";
import { WALRUS_PUBLISHER_URL } from "../config/globalVariables";

interface FileUploaderProps {
  onFileUpload: (file: File, blobId: string) => void;
  onFileRemove: () => void; // Callback for file removal
}

const FileUploader: React.FC<FileUploaderProps> = ({
  onFileUpload,
  onFileRemove,
}) => {
  const [uploadedFile, setUploadedFile] = useState<File | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isUploading, setIsUploading] = useState<boolean>(false); // Track upload status

  const handleDrop = async (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setErrorMessage(null);

    const file = e.dataTransfer.files[0];
    if (file && validateFile(file)) {
      await uploadToWalrus(file);
    }
  };

  const handleFileInputChange = async (
    e: React.ChangeEvent<HTMLInputElement>
  ) => {
    setErrorMessage(null);

    const file = e.target.files?.[0];
    if (file && validateFile(file)) {
      await uploadToWalrus(file);
    }
  };
  const cropToSquare = async (file: File): Promise<Blob> => {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const img = new Image();
        img.onload = () => {
          const size = Math.min(img.width, img.height); // Take the smallest dimension
          const canvas = document.createElement("canvas");
          const ctx = canvas.getContext("2d");

          if (!ctx) {
            reject(new Error("Canvas context not available"));
            return;
          }

          canvas.width = size;
          canvas.height = size;

          // Calculate cropping offsets
          const xOffset = (img.width - size) / 2;
          const yOffset = (img.height - size) / 2;

          // Draw cropped image onto the canvas
          ctx.drawImage(img, xOffset, yOffset, size, size, 0, 0, size, size);

          // Convert the canvas to a blob
          canvas.toBlob((blob) => {
            if (blob) resolve(blob);
            else reject(new Error("Failed to convert canvas to Blob"));
          }, file.type);
        };

        img.onerror = reject;
        img.src = reader.result as string;
      };

      reader.onerror = reject;
      reader.readAsDataURL(file);
    });
  };

  const validateFile = (file: File): boolean => {
    const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];
    const maxSize = 5 * 1024 * 1024; // 5 MB

    if (!allowedTypes.includes(file.type)) {
      setErrorMessage(
        "Invalid file type. Only jpeg, png, gif, and webp are allowed."
      );
      return false;
    }

    if (file.size > maxSize) {
      setErrorMessage("File size exceeds 5 MB.");
      return false;
    }

    return true;
  };

  const uploadToWalrus = async (file: File) => {
    setIsUploading(true);
    try {
      // Crop the image to a square
      const croppedBlob = await cropToSquare(file);

      const response = await fetch(
        `${WALRUS_PUBLISHER_URL}/v1/store?epochs=200`,
        {
          method: "PUT",
          body: croppedBlob,
        }
      );

      if (!response.ok) {
        throw new Error("Failed to upload file to Walrus");
      }

      const storageInfo = await response.json();
      const blobId = storageInfo.newlyCreated
        ? storageInfo.newlyCreated.blobObject.blobId
        : storageInfo.alreadyCertified.blobId;

      setUploadedFile(file); // Maintain original file reference for UI
      onFileUpload(file, blobId); // Notify parent with the new blobId
    } catch (error) {
      console.error("An error occurred while uploading:", error);
      setErrorMessage("Failed to upload file. Please try again.");
    } finally {
      setIsUploading(false);
    }
  };

  const handleRemoveFile = () => {
    setUploadedFile(null);
    setErrorMessage(null);
    onFileRemove(); // Notify parent of file removal
  };

  return (
    <div>
      {!uploadedFile ? (
        <div
          onDrop={handleDrop}
          onDragOver={(e) => e.preventDefault()}
          className="border border-dashed border-gray-600 rounded-lg p-4 mb-4 text-center cursor-pointer"
        >
          {isUploading ? (
            <p className="text-sm text-gray-500">Uploading to Walrus...</p>
          ) : (
            <>
              <img
                src="/Upload_Icon.png"
                alt="Upload Icon"
                className="w-12 h-12 mx-auto mb-2"
              />
              <p>
                Drag and drop a file here or{" "}
                <label
                  htmlFor="fileInput"
                  className="text-accent underline cursor-pointer"
                >
                  Choose File
                </label>
              </p>
              <p className="text-sm text-gray-500">jpeg, png, gif, webp</p>
              <p className="text-sm text-gray-500">5 MB max</p>
              <input
                type="file"
                id="fileInput"
                className="hidden"
                onChange={handleFileInputChange}
              />
            </>
          )}
        </div>
      ) : (
        <div className="relative text-center">
          <button
            onClick={handleRemoveFile}
            className="absolute top-[-5px] right-[-5px] text-white flex items-center justify-center z-50"
            aria-label="Remove file"
          >
            <img src="/X_Button.png" alt="Close Icon" className="w-6 h-6" />
          </button>

          <div className="relative w-full aspect-square rounded-lg overflow-hidden">
            <img
              src={URL.createObjectURL(uploadedFile)}
              alt="Uploaded"
              className="w-full h-full object-cover"
            />
            <div className="absolute bottom-[-2px] left-0 w-full bg-black opacity-98 text-white py-2 text-2xl font-ppNeueBit flex items-center justify-center space-x-2 rounded-b-lg">
              <span>Reliably Yours</span>
              <img src="/Heart_Icon.png" alt="Heart Icon" className="w-7 h-7" />
              <span>Walrus</span>
            </div>
          </div>
        </div>
      )}

      {errorMessage && (
        <p className="text-red-500 text-center mt-2">{errorMessage}</p>
      )}
    </div>
  );
};

export default FileUploader;
